import { listStudents, createStudent, updateStudent } from '$lib/db/repos/students';
import { isLearnerName } from '$lib/features/excel/roster';
import { normalizeLearnerName } from '../logic';
import { matchRosterLearner } from '../month/students';
import type { Sf2LearnerMatch, Sf2MonthStudentMapping } from '../month/students';
import type { Sf2WorkbookLearner } from '../calendar';
import type { Sf2StudentMappingRecord } from '../repository';
import type { StudentGender, StudentRecord } from '$lib/domain/models';

/**
 * Turning the learner names a teacher typed, or a workbook holds, into students.
 *
 * The rule this file exists to hold is the order a learner is identified in:
 * `sf2_learner_id` → `normalized_name` → `row_index`, which is exactly what
 * {@link matchRosterLearner} implements. A learner ID is the school's own record and
 * survives a rename *and* a re-sort; a name survives a row move; a row index says
 * nothing about the learner at all, and matching on it alone is what re-points a
 * student's X marks at somebody else when the roster is reshuffled between two files
 * (E7). So the row is the last resort, and it is logged when it is reached.
 */

/** What a workbook-learner sync did, and what the caller reports back to the teacher. */
export interface WorkbookLearnerSync {
	studentMappings: Sf2StudentMappingRecord[];
	studentsCreated: number;
	studentsReused: number;
	studentsUpdated: number;
}

/** The students a draft's learner names resolve to, and how each was arrived at. */
export interface DraftStudents {
	students: StudentRecord[];
	created: number;
	reused: number;
}

/**
 * Create the students for learner names the teacher typed into the draft.
 *
 * An existing student is matched on the normalized name, so re-running a draft is a
 * no-op rather than a second class full of the same children. A draft with no usable
 * learner name leaves the class exactly as it was and reports every existing student
 * as reused.
 */
export async function rosterStudentsForDraft(
	classId: string,
	learnerNames: readonly string[]
): Promise<DraftStudents> {
	const existingStudents = await listStudents(classId);
	// A duplicate normalized name in the class keeps the last one, which is what the
	// Rust `HashMap` did.
	const existingByName = new Map<string, StudentRecord>(
		existingStudents.map((student) => [normalizeLearnerName(student.name), student])
	);

	const requested: string[] = [];
	const seen = new Set<string>();
	for (const raw of learnerNames) {
		const name = raw.trim();
		if (name === '' || !isLearnerName(name)) continue;
		const normalized = normalizeLearnerName(name);
		if (seen.has(normalized)) continue;
		seen.add(normalized);
		requested.push(name);
	}

	if (requested.length === 0) {
		return { students: existingStudents, created: 0, reused: existingStudents.length };
	}

	const students: StudentRecord[] = [];
	let created = 0;
	let reused = 0;

	for (const name of requested) {
		const normalized = normalizeLearnerName(name);
		const existing = existingByName.get(normalized);
		if (existing !== undefined) {
			reused += 1;
			students.push(existing);
			continue;
		}
		// Gender is deliberately left unset: nothing the teacher typed into the draft
		// says which block a new learner belongs in, and guessing would silently put a
		// child in the wrong half of the form.
		const made = await createStudent({ name, classId });
		existingByName.set(normalized, made);
		created += 1;
		students.push(made);
	}

	return { students, created, reused };
}

/** The gender block a workbook learner row sits in, as a stored gender. */
function learnerGender(genderBlock: string | undefined): StudentGender | undefined {
	const block = genderBlock?.trim().toUpperCase();
	if (block === 'MALE') return 'male';
	if (block === 'FEMALE') return 'female';
	return undefined;
}

/**
 * Sync the workbook's learner rows onto the class's students, with no reference
 * template to fall back on.
 *
 * See {@link syncWorkbookLearnerMappingsWithOld}, which is the same thing with the
 * previous template's mappings supplied.
 */
export async function syncWorkbookLearnerMappings(
	classId: string,
	templateId: string,
	learners: readonly Sf2WorkbookLearner[]
): Promise<WorkbookLearnerSync> {
	return syncWorkbookLearnerMappingsWithOld(classId, templateId, learners, []);
}

/**
 * Sync workbook learner mappings, falling back to the previous template's mappings.
 *
 * When `oldMappings` is non-empty (a re-import), a learner that matches no student in
 * the class by name falls through to the previous file's mappings. A match there
 * updates the existing student's name to the workbook's rather than creating a
 * duplicate: the workbook is authoritative about a learner the app already knows - the
 * same learner, spelled the way the school now spells it.
 */
export async function syncWorkbookLearnerMappingsWithOld(
	classId: string,
	templateId: string,
	learners: readonly Sf2WorkbookLearner[],
	oldMappings: readonly Sf2StudentMappingRecord[]
): Promise<WorkbookLearnerSync> {
	const existingStudents = await listStudents(classId);
	const existingById = new Map<string, StudentRecord>(
		existingStudents.map((student) => [student.id, student])
	);
	const existingByName = new Map<string, StudentRecord>(
		existingStudents.map((student) => [normalizeLearnerName(student.name), student])
	);

	const reference = referenceMappings(oldMappings, existingById);

	const claimedStudents = new Set<string>();
	const seenNames = new Set<string>();
	const studentMappings: Sf2StudentMappingRecord[] = [];
	let studentsCreated = 0;
	let studentsReused = 0;
	let studentsUpdated = 0;

	for (const learner of learners) {
		// The form's own header and TOTAL rows live in the same column, so only a
		// learner-shaped name can claim a row.
		if (!isLearnerName(learner.name)) continue;
		const normalizedName = normalizeLearnerName(learner.name);
		if (seenNames.has(normalizedName)) continue;
		seenNames.add(normalizedName);

		const gender = learnerGender(learner.genderBlock);
		let student: StudentRecord | undefined = existingByName.get(normalizedName);

		if (student !== undefined) {
			studentsReused += 1;
			if (gender !== undefined && student.gender !== gender) {
				student = await updateStudent(student.id, { gender });
				existingByName.set(normalizedName, student);
			}
		} else {
			const matched = unclaimedMatch(reference, learner, claimedStudents);
			if (matched !== undefined) {
				if (matched.matchedBy === 'rowIndex') {
					console.warn(
						`roster sync: \`${learner.name.trim()}\` at row ${learner.rowIndex} matched an ` +
							'existing student by position only; a reshuffled roster can attach one ' +
							"student's X marks to another"
					);
				}
				student = existingById.get(matched.studentId);
				if (student !== undefined) {
					student = await updateStudent(student.id, {
						name: learner.name.trim(),
						...(gender === undefined ? {} : { gender })
					});
					existingByName.set(normalizeLearnerName(student.name), student);
					studentsUpdated += 1;
				}
			}
			if (student === undefined) {
				student = await createStudent({
					name: learner.name,
					...(gender === undefined ? {} : { gender }),
					classId
				});
				existingByName.set(normalizedName, student);
				studentsCreated += 1;
			}
		}

		claimedStudents.add(student.id);
		studentMappings.push({
			templateId,
			studentId: student.id,
			workbookName: learner.name,
			normalizedName,
			rowIndex: learner.rowIndex,
			genderBlock: learner.genderBlock
		});
	}

	return { studentMappings, studentsCreated, studentsReused, studentsUpdated };
}

/**
 * The previous template's mappings, in the shape {@link matchRosterLearner} reads,
 * with each learner's DepEd ID filled in from the student record.
 *
 * The legacy mappings have no ID of their own - the DepEd ID was never read out of a
 * workbook before spec §6.3 - so without this the `sf2_learner_id` branch of the
 * matcher could never fire on the re-import path and the promised order was really
 * just `normalized_name` → `row_index`. The ID lives on `students`, so the student's
 * own record is where the reference has to read it from.
 *
 * A student with no ID contributes `undefined`, and the matcher falls through to the
 * name - which is the correct outcome on the bundled template, whose merged
 * "No."/ID cell means the school never gives us a real one.
 */
function referenceMappings(
	oldMappings: readonly Sf2StudentMappingRecord[],
	studentsById: ReadonlyMap<string, StudentRecord>
): Sf2MonthStudentMapping[] {
	return oldMappings.map((mapping) => ({
		templateId: mapping.templateId,
		studentId: mapping.studentId,
		workbookName: mapping.workbookName,
		normalizedName: mapping.normalizedName,
		rowIndex: mapping.rowIndex,
		genderBlock: mapping.genderBlock,
		sf2LearnerId: studentsById.get(mapping.studentId)?.sf2LearnerId
	}));
}

/**
 * The first matcher result that is not already spoken for in this run.
 *
 * `sf2_student_mappings` keys on `(template_id, student_id)`, so two workbook rows
 * that both resolve to one student would collide. A reshuffled roster can produce
 * exactly that, and dropping the second match here is what keeps it from becoming a
 * duplicate student or a failed insert.
 *
 * A mapping whose `student_id` names no student in this class cannot identify one
 * either, so it is dropped too - the learner is created rather than guessed at.
 */
function unclaimedMatch(
	reference: readonly Sf2MonthStudentMapping[],
	learner: Sf2WorkbookLearner,
	claimedStudents: ReadonlySet<string>
): Sf2LearnerMatch | undefined {
	const match = matchRosterLearner([...reference], learner);
	if (match === undefined || claimedStudents.has(match.studentId)) return undefined;
	return reference.some((mapping) => mapping.studentId === match.studentId) ? match : undefined;
}
