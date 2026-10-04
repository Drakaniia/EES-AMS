import { invalidInput } from '$lib/db';
import type { Student } from '$lib/domain/models';
import {
	bundledTemplateTotalRows,
	rosterExpansionNeeded,
	SF2_FRESH_FEMALE_SLOTS,
	SF2_FRESH_FEMALE_START_ROW,
	SF2_FRESH_MALE_SLOTS,
	SF2_FIRST_LEARNER_ROW,
	SF2_ITEM_NUMBER_COLUMN,
	SF2_NAME_COLUMN
} from '$lib/features/excel/constants';
import { columnLetter } from '$lib/features/excel/workbook';
import { normalizeLearnerName } from '../logic';
import type { Sf2CellMark } from '$lib/features/excel/types';
import type { Sf2StudentMappingRecord } from '../repository';

/**
 * Roster slot arithmetic: which workbook row each student sits on, and the marks
 * that put them there.
 *
 * The row arithmetic is the whole risk of this file. One row off and the TOTAL rows,
 * the `COUNTIF` ranges and the teacher's printed report all shift together, silently,
 * so every number here is derived from the slot layout rather than restated.
 */

export { bundledTemplateTotalRows, rosterExpansionNeeded };
export type { Sf2TotalRows } from '$lib/features/excel/formula-marks';

/** One learner slot of the form: a row and the gender block it belongs to. */
export interface TemplateRosterSlot {
	rowIndex: number;
	genderBlock: 'MALE' | 'FEMALE';
}

/** A student, and the row they have been given. */
export interface TemplateRosterAssignment {
	student: Student;
	slot: TemplateRosterSlot;
}

const MALE: TemplateRosterSlot['genderBlock'] = 'MALE';
const FEMALE: TemplateRosterSlot['genderBlock'] = 'FEMALE';

function range(first: number, last: number): number[] {
	const rows: number[] = [];
	for (let row = first; row <= last; row += 1) rows.push(row);
	return rows;
}

/**
 * The slots a fresh bundled template ships with: 21 male rows and 19 female rows.
 *
 * The MALE TOTAL formula sits at row 29 and the FEMALE TOTAL at 49, so neither of
 * those rows is a slot and the two blocks are not contiguous.
 */
export function templateRosterSlots(): TemplateRosterSlot[] {
	return [
		...range(SF2_FIRST_LEARNER_ROW, SF2_FIRST_LEARNER_ROW + SF2_FRESH_MALE_SLOTS - 1).map(
			(rowIndex) => ({ rowIndex, genderBlock: MALE })
		),
		...range(
			SF2_FRESH_FEMALE_START_ROW,
			SF2_FRESH_FEMALE_START_ROW + SF2_FRESH_FEMALE_SLOTS - 1
		).map((rowIndex) => ({ rowIndex, genderBlock: FEMALE }))
	];
}

/**
 * Slot definitions for a roster larger than the template's 21 + 19 capacity.
 *
 * The female block starts after the MALE TOTAL row, which the male growth has pushed
 * down, so it starts at `30 + extraMale` rather than at 30.
 */
export function expandedRosterSlots(maleCount: number, femaleCount: number): TemplateRosterSlot[] {
	const { extraMale } = rosterExpansionNeeded(maleCount, femaleCount);
	return [
		...range(SF2_FIRST_LEARNER_ROW, SF2_FIRST_LEARNER_ROW + maleCount - 1).map((rowIndex) => ({
			rowIndex,
			genderBlock: MALE
		})),
		...range(
			SF2_FRESH_FEMALE_START_ROW + extraMale,
			SF2_FRESH_FEMALE_START_ROW + extraMale + femaleCount - 1
		).map((rowIndex) => ({ rowIndex, genderBlock: FEMALE }))
	];
}

/**
 * Whether the template owns the roster, and the app may therefore re-assign every
 * student to a slot itself.
 *
 * A bundled working copy carries a `bundled-<hash>-<classId>` source hash; anything
 * else is a workbook the school handed over, whose existing rows are the school's
 * arrangement and are not ours to overwrite.
 */
export function templateOwnsRoster(template: { sourceHash: string }): boolean {
	return template.sourceHash.startsWith('bundled-');
}

/**
 * Give every student a row, in male-then-female order.
 *
 * A student with no gender cannot be placed - the form has one block per gender and
 * no third block - so the whole operation is refused rather than dropping them.
 */
export function templateRosterAssignments(
	students: readonly Student[]
): TemplateRosterAssignment[] {
	const male: Student[] = [];
	const female: Student[] = [];
	const missingGender: string[] = [];

	for (const student of students) {
		if (student.gender === 'male') male.push(student);
		else if (student.gender === 'female') female.push(student);
		else missingGender.push(student.name.trim());
	}

	if (missingGender.length > 0) {
		throw invalidInput(
			`Set Male/Female for these students before creating or updating the SF2 workbook: ${missingGender.join(', ')}`
		);
	}

	const { extraMale, extraFemale } = rosterExpansionNeeded(male.length, female.length);
	const slots =
		extraMale > 0 || extraFemale > 0
			? expandedRosterSlots(male.length, female.length)
			: templateRosterSlots();

	const maleSlots = slots.filter((slot) => slot.genderBlock === MALE);
	const femaleSlots = slots.filter((slot) => slot.genderBlock === FEMALE);
	return [...zip(male, maleSlots), ...zip(female, femaleSlots)].map(({ student, slot }) => ({
		student,
		slot
	}));
}

function zip(
	students: readonly Student[],
	slots: readonly TemplateRosterSlot[]
): { student: Student; slot: TemplateRosterSlot }[] {
	return students.slice(0, slots.length).map((student, index) => ({ student, slot: slots[index] }));
}

/**
 * A normalized name no other learner on this sheet claims.
 *
 * `sf2_student_mappings` is keyed on the name, so two learners typed identically
 * would collapse into one. The second one keeps its name plus `#<studentId>`, which
 * is unique by construction.
 */
export function uniqueNormalizedName(seen: Set<string>, name: string, suffix: string): string {
	const normalized = normalizeLearnerName(name);
	if (!seen.has(normalized)) {
		seen.add(normalized);
		return normalized;
	}
	const unique = `${normalized}#${suffix}`;
	seen.add(unique);
	return unique;
}

/**
 * Refuse a roster that holds the same learner twice.
 *
 * The two would occupy two rows in the workbook and two `COUNTIF` ranges, and every
 * per-day count would then be one short. This is caught before a workbook is written
 * rather than after, because a printed SF2 with a missing count is a resubmission.
 */
export function rejectDuplicateRosterNames(students: readonly Student[]): void {
	const namesByNormalized = new Map<string, string[]>();
	for (const student of students) {
		const normalized = normalizeLearnerName(student.name);
		namesByNormalized.set(normalized, [...(namesByNormalized.get(normalized) ?? []), student.name]);
	}

	// Rust collected these into a `HashMap`, so its order was arbitrary. First
	// appearance is both deterministic and the order the teacher reads the roster in.
	const duplicates = [...namesByNormalized.values()]
		.filter((names) => names.length > 1)
		.map((names) => names.join(', '));
	if (duplicates.length === 0) return;

	throw invalidInput(
		`Duplicate learner names must be corrected before creating an SF2 workbook: ${duplicates.join('; ')}`
	);
}

/** The `No.` rows the TOTAL rows of a fresh template sit on, for an expansion insert. */
/**
 * Marks for the learner names (`C`) and the item numbers (`A`), per sheet.
 *
 * The item number restarts at 1 in each gender block, which is what the DepEd form
 * prints: a class is numbered 1..n for the boys and 1..m for the girls.
 */
export function rosterNameMarks(
	sheetNames: readonly string[],
	assignments: readonly TemplateRosterAssignment[]
): Sf2CellMark[] {
	const marks: Sf2CellMark[] = [];
	for (const sheetName of sheetNames) {
		let maleNumber = 0;
		let femaleNumber = 0;
		for (const { student, slot } of assignments) {
			const sequence = slot.genderBlock === MALE ? (maleNumber += 1) : (femaleNumber += 1);
			marks.push(
				{
					sheetName,
					address: `${columnLetter(SF2_ITEM_NUMBER_COLUMN)}${slot.rowIndex}`,
					value: String(sequence)
				},
				{
					sheetName,
					address: `${columnLetter(SF2_NAME_COLUMN)}${slot.rowIndex}`,
					value: student.name.trim()
				}
			);
		}
	}
	return marks;
}

/** The `sf2_student_mappings` rows a set of assignments describes. */
export function studentMappingsFromRosterAssignments(
	templateId: string,
	assignments: readonly TemplateRosterAssignment[]
): Sf2StudentMappingRecord[] {
	const seen = new Set<string>();
	return assignments.map(({ student, slot }) => ({
		templateId,
		studentId: student.id,
		workbookName: student.name,
		normalizedName: uniqueNormalizedName(seen, student.name, student.id),
		rowIndex: slot.rowIndex,
		genderBlock: slot.genderBlock
	}));
}
