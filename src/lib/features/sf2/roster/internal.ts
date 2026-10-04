import { invalidInput } from '$lib/db';
import { openWorkbook, saveWorkbookAtomic, sf2MonthlySheets } from '$lib/features/excel/workbook';
import { expandRosterRows, hideEmptyLearnerRows } from '$lib/features/excel/roster';
import {
	clearTotalRows,
	writeFormulaMarks,
	writeMarks,
	writeMarksForce
} from '$lib/features/excel/marks';
import {
	SF2_FRESH_FEMALE_SLOTS,
	SF2_FRESH_FEMALE_START_ROW,
	SF2_FRESH_MALE_SLOTS,
	SF2_FIRST_LEARNER_ROW
} from '$lib/features/excel/constants';
import { dateMappingsFromAnalysis } from '../metadata';
import { layoutFingerprint } from '../workbook-files';
import { normalizeLearnerName } from '../logic';
import { updateTemplateWithMappings } from '../repository';
import { clearUnusedLearnerMarks, genderCounts } from './helpers';
import { readWorkbookAnalysis } from './analysis';
import { rosterSyncFormulaMarks, mappedSheetNames } from './formula-marks';
import {
	bundledTemplateTotalRows,
	rosterNameMarks,
	studentMappingsFromRosterAssignments,
	templateRosterAssignments
} from './parser';
import type { Sf2WorkbookLearner } from '../calendar';
import type { Sf2StudentMappingRecord, Sf2TemplateRecord } from '../repository';
import type { Student } from '$lib/domain/models';
import type { Sf2CellMark } from '$lib/features/excel/types';

/**
 * The two roster-sync branches.
 *
 * Which branch runs is decided by whether the template owns the roster, and it is the
 * single most important decision on this path:
 *
 * - A **bundled** working copy belongs to the app. Every student can be re-assigned to
 *   any row, and the workbook grows when the class is larger than its slots.
 * - An **imported** workbook is the school's. Its existing rows are the school's
 *   arrangement and are never overwritten; a new student can only go into a row the
 *   school left free, which is why a class that outgrows the file is refused here
 *   rather than silently overwriting somebody.
 *
 * There is no Excel session any more. Each branch opens the workbook, makes every
 * change in memory, and writes once — which is what the Rust `batch_operations`
 * wrapper was already emulating, minus the process per call.
 */

/** Re-assign every student to a slot, growing the workbook when the class outgrows it. */
export async function syncBundledTemplateRoster(
	template: Sf2TemplateRecord,
	students: readonly Student[],
	existingMappings: readonly Sf2StudentMappingRecord[]
): Promise<Sf2TemplateRecord> {
	const { maleCount, femaleCount } = genderCounts(students);
	const existingMaleMapped = countBlock(existingMappings, 'MALE');
	const existingFemaleMapped = countBlock(existingMappings, 'FEMALE');

	// The capacity the workbook already has, which is what decides how far it has to
	// grow. An already-expanded workbook is not expanded a second time.
	const currentMaleCapacity = Math.max(existingMaleMapped, SF2_FRESH_MALE_SLOTS);
	const currentFemaleCapacity = Math.max(existingFemaleMapped, SF2_FRESH_FEMALE_SLOTS);
	const extraMale = Math.max(0, maleCount - currentMaleCapacity);
	const extraFemale = Math.max(0, femaleCount - currentFemaleCapacity);

	const assignments = templateRosterAssignments(students);
	const rows = bundledTemplateTotalRows(maleCount, femaleCount);

	// The TOTAL rows the insert has to land above. A workbook with no mappings yet is
	// a fresh template, whose TOTAL rows are the fixed 29 and 49.
	const isFresh = existingMappings.length === 0;
	const currentMaleTotal = SF2_FIRST_LEARNER_ROW + currentMaleCapacity;
	const currentFemaleTotal =
		SF2_FRESH_FEMALE_START_ROW +
		(currentMaleCapacity - SF2_FRESH_MALE_SLOTS) +
		currentFemaleCapacity;

	// Pure arithmetic, no workbook: the mappings are known before the file is opened.
	const studentMappings = studentMappingsFromRosterAssignments(template.id, assignments);

	const workbook = await openWorkbook(template.sourcePath);
	if (extraMale > 0 || extraFemale > 0) {
		expandRosterRows(
			workbook,
			extraMale,
			extraFemale,
			isFresh ? 29 : currentMaleTotal,
			isFresh ? 49 : currentFemaleTotal
		);
	}

	const analysis = readWorkbookAnalysis(workbook);
	const sheetNames = sf2MonthlySheets(workbook).map((sheet) => sheet.name);

	writeMarks(workbook, rosterNameMarks(sheetNames, assignments));

	const mappedRows = assignments.map((assignment) => assignment.slot.rowIndex);
	const expanded = extraMale > 0 || extraFemale > 0;
	const clearMarks = clearUnusedLearnerMarks(
		sheetNames,
		mappedRows,
		expanded ? maleCount : undefined,
		expanded ? femaleCount : undefined
	);
	if (clearMarks.length > 0) writeMarks(workbook, clearMarks);

	hideEmptyLearnerRows(workbook, rows.maleTotalRow, rows.femaleTotalRow, new Set(mappedRows));

	const dateMappings = dateMappingsFromAnalysis(template.id, analysis);
	for (const sheetName of mappedSheetNames(dateMappings)) clearTotalRows(workbook, sheetName, rows);

	// Best-effort, as in Rust: a formula that cannot be written must not cost the
	// teacher the roster they just saved.
	const marks = rosterSyncFormulaMarks(workbook, maleCount, femaleCount, rows, dateMappings);
	bestEffort(() => writeFormulaMarks(workbook, marks.totalMarks), 'TOTAL formula');
	bestEffort(() => writeFormulaMarks(workbook, marks.summaryFormulaMarks), 'summary formula');
	bestEffort(() => writeMarksForce(workbook, marks.summaryStaticMarks), 'summary static');

	await saveWorkbookAtomic(workbook, template.sourcePath);

	const synced: Sf2TemplateRecord = {
		...template,
		layoutFingerprint: layoutFingerprint(analysis)
	};
	await updateTemplateWithMappings(synced, studentMappings, dateMappings);
	return synced;
}

/**
 * Map new database students onto the empty rows of a workbook the school handed over.
 *
 * A workbook with no free row for a new student is refused, and the message says what
 * to do about it: add rows in Excel and import again. That is the only honest answer,
 * because the alternative is taking over a row somebody already occupies.
 */
export async function syncImportedWorkbookRoster(
	template: Sf2TemplateRecord,
	students: readonly Student[],
	existingMappings: readonly Sf2StudentMappingRecord[]
): Promise<Sf2TemplateRecord> {
	const workbook = await openWorkbook(template.sourcePath);
	const analysis = readWorkbookAnalysis(workbook);

	const mappedRows = new Set(existingMappings.map((mapping) => mapping.rowIndex));
	const unmappedMaleRows = analysis.learners.filter(
		(learner) => learner.genderBlock === 'MALE' && !mappedRows.has(learner.rowIndex)
	);
	const unmappedFemaleRows = analysis.learners.filter(
		(learner) => learner.genderBlock === 'FEMALE' && !mappedRows.has(learner.rowIndex)
	);

	const mappedStudents = new Set(existingMappings.map((mapping) => mapping.studentId));
	const newMale = students.filter(
		(student) => student.gender === 'male' && !mappedStudents.has(student.id)
	);
	const newFemale = students.filter(
		(student) => student.gender === 'female' && !mappedStudents.has(student.id)
	);

	// Every student already sits in a row: nothing to do, and the template is
	// returned untouched so a no-op sync does not restamp its fingerprint.
	if (newMale.length === 0 && newFemale.length === 0) return { ...template };

	if (newMale.length > unmappedMaleRows.length || newFemale.length > unmappedFemaleRows.length) {
		const totalLearnerSlots =
			existingMappings.length + unmappedMaleRows.length + unmappedFemaleRows.length;
		throw invalidInput(
			`The imported SF2 workbook has ${totalLearnerSlots} learner rows in total, but this class now has ${students.length} learners. ` +
				'Open the workbook in Excel, add rows for the extra learners, then import the workbook again.'
		);
	}

	const seen = new Set(existingMappings.map((mapping) => mapping.normalizedName));
	const newMappings: Sf2StudentMappingRecord[] = [];
	const nameMarks: Sf2CellMark[] = [];
	assignStudentsToRows(
		template.id,
		sf2MonthlySheets(workbook).map((sheet) => sheet.name),
		seen,
		newMale,
		unmappedMaleRows,
		newMappings,
		nameMarks
	);
	assignStudentsToRows(
		template.id,
		sf2MonthlySheets(workbook).map((sheet) => sheet.name),
		seen,
		newFemale,
		unmappedFemaleRows,
		newMappings,
		nameMarks
	);

	if (nameMarks.length > 0) {
		writeMarks(workbook, nameMarks);
		await saveWorkbookAtomic(workbook, template.sourcePath);
	}

	// Re-read after the write: the mappings have to describe the file as it now is.
	const refreshed = readWorkbookAnalysis(workbook);
	const dateMappings = dateMappingsFromAnalysis(template.id, refreshed);
	const allMappings = [...existingMappings, ...newMappings];

	const synced: Sf2TemplateRecord = {
		...template,
		layoutFingerprint: layoutFingerprint(refreshed)
	};
	await updateTemplateWithMappings(synced, allMappings, dateMappings);

	console.info(
		`Roster sync for imported workbook '${template.id}': added ` +
			`${newMappings.length} new student(s) (${newMale.length} male, ${newFemale.length} female)`
	);
	return synced;
}

/**
 * Give each new student a free learner row, and the name mark that puts them there.
 *
 * Only column `C` is written: the workbook's own `No.` column belongs to the school's
 * arrangement, and renumbering it would move every name below it.
 */
function assignStudentsToRows(
	templateId: string,
	sheetNames: readonly string[],
	seenNormalizedNames: Set<string>,
	newStudents: readonly Student[],
	unmappedRows: readonly Sf2WorkbookLearner[],
	newMappings: Sf2StudentMappingRecord[],
	nameMarks: Sf2CellMark[]
): void {
	for (const [index, student] of newStudents.entries()) {
		const learnerRow = unmappedRows[index];
		if (learnerRow === undefined) continue;
		const normalizedName = normalizeLearnerName(student.name);
		const uniqueName = seenNormalizedNames.has(normalizedName)
			? `${normalizedName}#${student.id}`
			: normalizedName;
		seenNormalizedNames.add(uniqueName);

		newMappings.push({
			templateId,
			studentId: student.id,
			workbookName: student.name,
			normalizedName: uniqueName,
			rowIndex: learnerRow.rowIndex,
			genderBlock: learnerRow.genderBlock
		});

		for (const sheetName of sheetNames) {
			nameMarks.push({
				sheetName,
				address: `C${learnerRow.rowIndex}`,
				value: student.name.trim()
			});
		}
	}
}

function countBlock(
	mappings: readonly Sf2StudentMappingRecord[],
	block: 'MALE' | 'FEMALE'
): number {
	return mappings.filter((mapping) => mapping.genderBlock === block).length;
}

function bestEffort(write: () => void, what: string): void {
	try {
		write();
	} catch (thrown) {
		const reason = thrown instanceof Error ? thrown.message : String(thrown);
		throw new Error(`failed to write ${what} marks during roster sync: ${reason}`, {
			cause: thrown
		});
	}
}
