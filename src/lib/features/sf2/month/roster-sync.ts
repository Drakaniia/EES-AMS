/**
 * Putting a roster change onto a class's month worksheets.
 *
 * The Students page calls this after every add, edit and delete, so a learner is on
 * the SF2 grid the moment they are on file. It is the piece the per-month model was
 * missing: `sf2_month_student_mappings` was only ever written when the twelve-sheet
 * workbook was first split, so every student added afterwards was invisible to the
 * Reports grid and to every mark the app tried to write for them.
 *
 * ## The roster is one roster
 *
 * Every month of a class is a worksheet of the *same* file, so a learner sits on one
 * row number across all twelve. A sync therefore decides the row assignment once and
 * gives it to every month, rather than letting each month drift into its own
 * arrangement - the first month that has a roster decides, and the rest agree.
 *
 * ## Rows are kept, not re-assigned
 *
 * A row already holding a learner keeps that learner: the `X` marks on it are a
 * teacher's record of that child's absences, and moving the row would move them.
 * New learners take the free rows above the MALE TOTAL / FEMALE TOTAL, and a block
 * grows only when it has run out - `expandRosterRows` splices every month sheet at
 * once so the twelve stay laid out identically.
 */

import { invalidInput } from '$lib/db';
import { listStudents } from '$lib/db/repos/students';
import { getFileSystem } from '$lib/platform/fs';
import {
	SF2_DEPED_LEARNER_ID_COLUMN,
	SF2_FIRST_LEARNER_ROW,
	SF2_ITEM_NUMBER_COLUMN,
	SF2_NAME_COLUMN
} from '$lib/features/excel/constants';
import type { Sf2TotalRows } from '$lib/features/excel/formula-marks';
import { expandRosterRows, hideEmptyLearnerRows } from '$lib/features/excel/roster';
import { writeFormulaMarks, writeMarksForce } from '$lib/features/excel/marks';
import type { Sf2CellMark } from '$lib/features/excel/types';
import {
	columnLetter,
	openWorkbook,
	saveWorkbookAtomic,
	sf2MonthlySheets
} from '$lib/features/excel/workbook';
import { monthWorkbookSheetName } from '$lib/features/sf2/workbook-files';
import type { Workbook, Worksheet } from 'exceljs';
import type { Student } from '$lib/domain/models';
import { monthFormulaMarks, totalsOn } from '$lib/features/sf2/attendance/attendance-write';
import { summaryMarksForRoster } from '../roster/formula-marks';
import { genderCounts } from '../roster/helpers';
import { rejectDuplicateRosterNames } from '../roster/parser';
import { normalizeLearnerName } from '../logic';
import { listAllMonthTemplates, listMonthDateMappings, setMonthLastSyncedAt } from './templates';
import { deleteMonthRoster, monthRosterForTemplate, replaceMonthRoster } from './students';
import type { Sf2MonthStudentMapping } from './students';
import type { Sf2MonthTemplate } from '$lib/types';

/** The columns a learner row is identified by, cleared when the row is vacated. */
const LEARNER_COLUMNS = [SF2_ITEM_NUMBER_COLUMN, SF2_DEPED_LEARNER_ID_COLUMN, SF2_NAME_COLUMN];

type Block = 'male' | 'female';

/** One learner, and the month-sheet row they sit on. */
interface Placement {
	student: Student;
	rowIndex: number;
	block: Block;
}

/** The learner slots of a sheet, per gender block. */
interface Slots {
	male: number[];
	female: number[];
}

/** The layout the placements were fitted into, which the insert may have moved. */
interface LaidOut {
	placements: Placement[];
	rows: Sf2TotalRows;
}

/**
 * Map `classId`'s students onto every month worksheet it has, and return how many
 * learners are now mapped.
 *
 * A class with no month worksheet on record is the normal state before the SF2
 * workbook has been imported, and is a no-op rather than an error. A workbook file
 * missing from disk *is* an error: writing mappings for a file nobody can open is
 * how marks end up going nowhere.
 */
export async function syncMonthRosterForClass(classId: string): Promise<number> {
	const templates = (await listAllMonthTemplates()).filter(
		(template) => template.classId === classId
	);
	if (templates.length === 0) return 0;

	const students = await listStudents(classId);
	rejectDuplicateRosterNames(students);
	// Deleting the last student clears every month roster instead of leaving the
	// old rows behind: `replaceMonthRoster` rejects an empty roster, so an early
	// return here is how a deleted learner stayed on the SF2 grid. The database
	// rows go first: the workbook clear below can fail (most often because the
	// file is open in Excel), and the retry is then a no-op on the mappings but
	// still rewrites the workbook.
	if (students.length === 0) {
		for (const template of templates) await deleteMonthRoster(template.id);
		for (const months of monthsPerWorkbook(templates)) {
			await clearWorkbookRoster(months[0].sourcePath);
		}
		return 0;
	}

	let placed = 0;
	for (const months of monthsPerWorkbook(templates)) {
		placed = await syncOneWorkbook(months, students);
	}
	return placed;
}

/** Every month's students placed on the worksheets of the one file they share. */
async function syncOneWorkbook(
	templates: Sf2MonthTemplate[],
	students: readonly Student[]
): Promise<number> {
	const sourcePath = templates[0].sourcePath;
	if (!(await getFileSystem().exists(sourcePath))) {
		throw invalidInput(`The SF2 workbook at ${sourcePath} is missing. Restore it from a backup.`);
	}

	const workbook = await openWorkbook(sourcePath);
	const sheets = sf2MonthlySheets(workbook);
	if (sheets.length === 0) return 0;

	const laidOut = placeStudents(
		students,
		await seededRoster(templates),
		totalsOn(sheets[0]),
		workbook
	);
	const { placements, rows } = laidOut;
	const rosterRows = new Set(placements.map((placement) => placement.rowIndex));

	writeRosterMarks(workbook, sheets, placements, rosterRows, rows);
	hideEmptyLearnerRows(workbook, rows.maleTotalRow, rows.femaleTotalRow, rosterRows);

	const { maleCount, femaleCount } = genderCounts(students);
	const now = Math.floor(Date.now() / 1000);
	for (const template of templates) {
		const roster = mappingsFor(template, placements);
		const formulas = monthFormulaMarks(workbook, {
			sourcePath,
			sheetName: sheetNameOf(template),
			roster,
			dates: await listMonthDateMappings(template.id)
		});
		writeFormulaMarks(workbook, formulas.formulaMarks);
		writeMarksForce(workbook, formulas.staticMarks);

		await replaceMonthRoster(template.id, roster);
		await setMonthLastSyncedAt(template.id, now);
	}

	// The Enrolment summary sits above the day grid and counts the whole class, so it
	// is written once per file rather than once per month.
	//
	// Best-effort, as the legacy roster sync is: a class with no girls at all puts a
	// `0` in a summary cell the form merges with its neighbour, and losing the
	// enrolment figure is a far smaller cost than losing the roster the teacher just
	// saved. Excel recomputes the block, and the next month build writes it outright.
	bestEffort(() => {
		const summary = summaryMarksForRoster(workbook, maleCount, femaleCount, rows, [
			...new Set(templates.map(sheetNameOf))
		]);
		writeFormulaMarks(workbook, summary.formulaMarks);
		writeMarksForce(workbook, summary.staticMarks);
	}, 'the Enrolment summary');

	await saveWorkbookAtomic(workbook, sourcePath);
	return placements.length;
}

/** The month's own worksheet, named the way the split named it. */
function sheetNameOf(template: Sf2MonthTemplate): string {
	return monthWorkbookSheetName(template.reportMonth, template.reportYear);
}

/** Months grouped by the workbook file they are worksheets of. */
function monthsPerWorkbook(templates: Sf2MonthTemplate[]): Sf2MonthTemplate[][] {
	const groups = new Map<string, Sf2MonthTemplate[]>();
	for (const template of templates) {
		const group = groups.get(template.sourcePath);
		if (group === undefined) groups.set(template.sourcePath, [template]);
		else group.push(template);
	}
	return [...groups.values()];
}

/**
 * Blank every learner row of one workbook's month sheets and hide them, so a
 * class with no students left prints no roster at all.
 *
 * Throws when the workbook is missing or cannot be saved, like the roster sync
 * it stands in for on this path: the caller reports it instead of undoing the
 * save, and the retry is a no-op on the already-cleared mappings.
 */
async function clearWorkbookRoster(sourcePath: string): Promise<void> {
	if (!(await getFileSystem().exists(sourcePath))) {
		throw invalidInput(`The SF2 workbook at ${sourcePath} is missing. Restore it from a backup.`);
	}

	const workbook = await openWorkbook(sourcePath);
	const sheets = sf2MonthlySheets(workbook);
	if (sheets.length === 0) return;

	const rows = totalsOn(sheets[0]);
	const slots = [...slotsOn(rows).male, ...slotsOn(rows).female];
	const marks: Sf2CellMark[] = [];
	for (const sheet of sheets) {
		for (const row of slots) {
			for (const column of LEARNER_COLUMNS) marks.push(cell(sheet, column, row, ''));
		}
	}
	// Forced: a cleared row can be the master of a merged pair, and the
	// item-number column is one of the pairs the form merges.
	writeMarksForce(workbook, marks);
	hideEmptyLearnerRows(workbook, rows.maleTotalRow, rows.femaleTotalRow, new Set());

	await saveWorkbookAtomic(workbook, sourcePath);
}

/** The roster the split left behind, taken from the first month that has one. */
async function seededRoster(
	templates: readonly Sf2MonthTemplate[]
): Promise<Sf2MonthStudentMapping[]> {
	for (const template of templates) {
		const roster = await monthRosterForTemplate(template.id);
		if (roster.length > 0) return roster;
	}
	return [];
}

/**
 * Give every student a row: the one they already hold, or the next free row of
 * their gender block. A block that has run out of rows is grown first, which moves
 * the TOTAL rows - so the grown layout is handed back to the caller rather than
 * assumed.
 */
function placeStudents(
	students: readonly Student[],
	seeded: readonly Sf2MonthStudentMapping[],
	rows: Sf2TotalRows,
	workbook: Workbook
): LaidOut {
	const slots = slotsOn(rows);
	const placements = new Map<string, Placement>();
	for (const mapping of seeded) {
		const block: Block = mapping.genderBlock?.toUpperCase() === 'FEMALE' ? 'female' : 'male';
		if (placements.has(mapping.studentId)) continue;
		// A row outside its own block's range belongs to a layout that no longer
		// exists, so it is treated as free rather than trusted.
		if (!slots[block].includes(mapping.rowIndex)) continue;
		const student = students.find((entry) => entry.id === mapping.studentId);
		if (student === undefined || blockOf(student) !== block) continue;
		placements.set(student.id, { student, rowIndex: mapping.rowIndex, block });
	}

	const claimed = {
		male: new Set(rowIndexes(placements, 'male')),
		female: new Set(rowIndexes(placements, 'female'))
	};
	const wanted = {
		male: countBlock(students, 'male'),
		female: countBlock(students, 'female')
	};
	// Bodies minus chairs: every claimed learner already sits in a slot, so the
	// claimed count cancels out and only the block headcount matters. Counting
	// `wanted - free` instead grows by the already-seated learners a second
	// time, splicing a block apart on every sync once a class is mapped.
	const missing = {
		male: Math.max(0, wanted.male - slots.male.length),
		female: Math.max(0, wanted.female - slots.female.length)
	};

	let grown = rows;
	if (missing.male > 0 || missing.female > 0) {
		expandRosterRows(
			workbook,
			missing.male,
			missing.female,
			rows.maleTotalRow,
			rows.femaleTotalRow
		);
		grown = totalsOn(sf2MonthlySheets(workbook)[0]);
		const expanded = slotsOn(grown);
		slots.male = expanded.male;
		slots.female = expanded.female;
	}

	for (const student of students) {
		if (placements.has(student.id)) continue;
		const block = blockOf(student);
		if (block === undefined) continue;
		const row = slots[block].find((candidate) => !claimed[block].has(candidate));
		if (row === undefined) continue;
		claimed[block].add(row);
		placements.set(student.id, { student, rowIndex: row, block });
	}

	return {
		placements: [...placements.values()].sort((left, right) => left.rowIndex - right.rowIndex),
		rows: grown
	};
}

/**
 * Write every learner's number and name onto every month sheet, and blank the rows
 * nobody sits in any more.
 *
 * The names go on every sheet rather than on each month's own, because a learner
 * whose row exists on one month and not another would have their absences land on
 * the wrong row the first time the teacher flips months.
 */
function writeRosterMarks(
	workbook: Workbook,
	sheets: readonly Worksheet[],
	placements: readonly Placement[],
	occupied: ReadonlySet<number>,
	rows: Sf2TotalRows
): void {
	const itemNumbers = numberByBlock(placements);
	const marks: Sf2CellMark[] = [];
	for (const sheet of sheets) {
		for (const placement of placements) {
			marks.push(
				cell(
					sheet,
					SF2_ITEM_NUMBER_COLUMN,
					placement.rowIndex,
					String(itemNumbers.get(placement.rowIndex))
				),
				cell(sheet, SF2_NAME_COLUMN, placement.rowIndex, placement.student.name.trim())
			);
		}
		for (const row of [...slotsOn(rows).male, ...slotsOn(rows).female]) {
			if (occupied.has(row)) continue;
			for (const column of LEARNER_COLUMNS) marks.push(cell(sheet, column, row, ''));
		}
	}
	// Forced: a cleared row can be the master of a merged pair, and the item-number
	// column is one of the pairs the form merges.
	writeMarksForce(workbook, marks);
}

/** Each learner's item number, counted from 1 within their own gender block. */
function numberByBlock(placements: readonly Placement[]): Map<number, number> {
	const numbers = new Map<number, number>();
	for (const block of ['male', 'female'] as const) {
		let item = 0;
		for (const placement of placements) {
			if (placement.block !== block) continue;
			item += 1;
			numbers.set(placement.rowIndex, item);
		}
	}
	return numbers;
}

/** One month's mappings, from the placements that are the same on every month. */
function mappingsFor(
	template: Sf2MonthTemplate,
	placements: readonly Placement[]
): Sf2MonthStudentMapping[] {
	const seen = new Set<string>();
	return placements.map((placement) => {
		// `sf2_month_student_mappings` is keyed on the normalized name, so a second
		// learner typed identically gets a suffix - the rule the legacy roster sync
		// already uses.
		let normalizedName = normalizeLearnerName(placement.student.name);
		if (seen.has(normalizedName)) normalizedName = `${normalizedName}#${placement.student.id}`;
		seen.add(normalizedName);
		return {
			templateId: template.id,
			studentId: placement.student.id,
			workbookName: placement.student.name,
			normalizedName,
			rowIndex: placement.rowIndex,
			genderBlock: placement.block === 'female' ? 'FEMALE' : 'MALE'
		};
	});
}

function blockOf(student: Student): Block | undefined {
	if (student.gender === 'male') return 'male';
	if (student.gender === 'female') return 'female';
	return undefined;
}

function countBlock(students: readonly Student[], block: Block): number {
	return students.filter((student) => blockOf(student) === block).length;
}

function rowIndexes(placements: ReadonlyMap<string, Placement>, block: Block): number[] {
	return [...placements.values()]
		.filter((placement) => placement.block === block)
		.map((placement) => placement.rowIndex);
}

/** The learner slots of a sheet, per gender block. */
function slotsOn(rows: Sf2TotalRows): Slots {
	return {
		male: range(SF2_FIRST_LEARNER_ROW, rows.maleTotalRow - 1),
		female: range(rows.maleTotalRow + 1, rows.femaleTotalRow - 1)
	};
}

function range(first: number, last: number): number[] {
	const rows: number[] = [];
	for (let row = first; row <= last; row += 1) rows.push(row);
	return rows;
}

function cell(sheet: Worksheet, column: number, row: number, value: string): Sf2CellMark {
	return { sheetName: sheet.name, address: `${columnLetter(column)}${row}`, value };
}

/** Run a write that must not cost the caller the whole sync when it cannot land. */
function bestEffort(write: () => void, what: string): void {
	try {
		write();
	} catch (thrown) {
		const reason = thrown instanceof Error ? thrown.message : String(thrown);
		console.warn(`Roster sync left ${what} alone: ${reason}`);
	}
}
