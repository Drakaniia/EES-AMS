import {
	SF2_DEPED_LEARNER_ID_COLUMN,
	SF2_FRESH_FEMALE_SLOTS,
	SF2_FRESH_FEMALE_START_ROW,
	SF2_FRESH_MALE_SLOTS,
	SF2_FRESH_MALE_TOTAL_ROW,
	SF2_FIRST_LEARNER_ROW,
	SF2_ITEM_NUMBER_COLUMN,
	SF2_NAME_COLUMN
} from '$lib/features/excel/constants';
import { columnLetter } from '$lib/features/excel/workbook';
import { createClass, listClasses } from '$lib/db/repos/classes';
import type { Class, Student } from '$lib/domain/models';
import type { Settings } from '$lib/types';
import type { Sf2CellMark } from '$lib/features/excel/types';

/**
 * The two helpers the roster path shares — the port of
 * `src-tauri/src/sf2/roster/roster_helpers.rs`.
 */

/** The standard columns of the SF2 learner info block: item number, LRN, name. */
const LEARNER_INFO_COLUMNS = [SF2_ITEM_NUMBER_COLUMN, SF2_DEPED_LEARNER_ID_COLUMN, SF2_NAME_COLUMN];

/** The learner slots a fresh bundled template ships with, as row numbers. */
function freshSlots(): number[] {
	const rows: number[] = [];
	for (
		let row = SF2_FIRST_LEARNER_ROW;
		row < SF2_FIRST_LEARNER_ROW + SF2_FRESH_MALE_SLOTS;
		row += 1
	) {
		rows.push(row);
	}
	const femaleStart = SF2_FRESH_MALE_TOTAL_ROW + 1;
	for (let row = femaleStart; row < femaleStart + SF2_FRESH_FEMALE_SLOTS; row += 1) rows.push(row);
	return rows;
}

/**
 * Marks that empty the learner rows nothing sits in, so a learner removed from the
 * class does not leave their name and marks in the printed form.
 *
 * The MALE and FEMALE TOTAL rows are not learner slots and are never cleared: they
 * carry the subtotals the roster sync has just written.
 *
 * `expandedMaleCount` / `expandedFemaleCount` are passed only when the workbook was
 * just grown, because then the slots run past row 28 / 48 and the fresh template's
 * own layout no longer says where they end.
 */
export function clearUnusedLearnerMarks(
	sheetNames: readonly string[],
	mappedRows: readonly number[],
	expandedMaleCount?: number,
	expandedFemaleCount?: number
): Sf2CellMark[] {
	const allPossibleRows =
		expandedMaleCount !== undefined && expandedFemaleCount !== undefined
			? expandedSlots(expandedMaleCount, expandedFemaleCount)
			: freshSlots();

	const mapped = new Set(mappedRows);
	const unusedRows = allPossibleRows.filter((row) => !mapped.has(row));
	if (unusedRows.length === 0) return [];

	const marks: Sf2CellMark[] = [];
	for (const sheetName of sheetNames) {
		for (const column of LEARNER_INFO_COLUMNS) {
			for (const row of unusedRows) {
				marks.push({ sheetName, address: `${columnLetter(column)}${row}`, value: '' });
			}
		}
	}
	return marks;
}

/** The learner slots of a just-grown roster, as row numbers. */
function expandedSlots(maleCount: number, femaleCount: number): number[] {
	const extraMale = Math.max(0, maleCount - SF2_FRESH_MALE_SLOTS);
	const rows: number[] = [];
	for (let row = SF2_FIRST_LEARNER_ROW; row < SF2_FIRST_LEARNER_ROW + maleCount; row += 1) {
		rows.push(row);
	}
	const femaleStart = SF2_FRESH_FEMALE_START_ROW + extraMale;
	for (let row = femaleStart; row < femaleStart + femaleCount; row += 1) rows.push(row);
	return rows;
}

/** How many students of each gender, which every row calculation starts from. */
export function genderCounts(students: readonly Student[]): {
	maleCount: number;
	femaleCount: number;
} {
	return {
		maleCount: students.filter((student) => student.gender === 'male').length,
		femaleCount: students.filter((student) => student.gender === 'female').length
	};
}

/**
 * Find a class by name, or create it if it does not exist yet.
 *
 * The comparison ignores case, because the class name is derived from the workbook's
 * own grade-level and section cells and a workbook the teacher typed by hand will not
 * match the stored casing exactly.
 *
 * A new class inherits the school's session times from settings where it has them.
 * Those are the app's defaults rather than the school's, so a class created this way
 * is one the teacher has to check on the Classes page - which is the same thing that
 * happened before the port.
 */
export async function findOrCreateClass(className: string, settings?: Settings): Promise<Class> {
	const classes = await listClasses();
	const existing = classes.find(
		(candidate) => candidate.name.toLowerCase() === className.toLowerCase()
	);
	if (existing !== undefined) return existing;

	return createClass({
		name: className,
		room: 'N/A',
		dayStart: settings?.dayStart ?? '08:00',
		dayEnd: settings?.dayEnd ?? '15:00',
		lateAfter: settings?.lateAfter ?? '08:45',
		sessions: [],
		days: [1, 2, 3, 4, 5]
	});
}
