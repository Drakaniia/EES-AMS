/**
 * A realistic install for the diagnose tests: a real DepEd template on an in-memory
 * file system, and a database seeded the way a teacher's `.sqlite` actually stands.
 *
 * `src-tauri/src/sf2/diagnose/__tests__/real_data_tests.rs` ran the diagnostic against
 * a live install, which needed a real database and real workbooks and so could not run
 * in CI. The intent of that test - the shape a real install has, and the fact that a
 * run writes nothing - is reproduced here from the real converted template plus a
 * seed written against the Rust test's own rows, so no Excel and no teacher data are
 * needed.
 *
 * The seed deliberately keeps the awkward parts of a real install: an absence recorded
 * against a different class, an absence with no class at all, an absence for a student
 * of another class, a non-`absent` event, and *both* the legacy and the per-month
 * mapping tables populated.
 */

import ExcelJS from 'exceljs';
import type { Worksheet } from 'exceljs';
import { loadTemplate, type TemplateFixture } from '$lib/features/excel/__tests__/template-fixture';
import {
	cellText,
	columnNumber,
	openWorkbook,
	saveWorkbookAtomic,
	setCellText,
	writableDayColumns
} from '$lib/features/excel/workbook';
import {
	SF2_DAY_ROW,
	SF2_FIRST_LEARNER_ROW,
	SF2_FRESH_FEMALE_SLOTS,
	SF2_FRESH_FEMALE_START_ROW
} from '$lib/features/excel/constants';
import { useSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import { useDriver } from '$lib/db';
import { NodeSqlDriver } from '$lib/db/node-driver';
import type { SqlDriver } from '$lib/db';

export const WORKBOOK_DIR = '/Documents/EES-AMS/workbooks';
export const LEGACY_WORKBOOK_PATH = `${WORKBOOK_DIR}/_legacy/SF2-GRADE-3-MATAPAT-0000t1`;
export const CLASS_ID = 'c1';
export const TEMPLATE_ID = 't1';
export const SCHOOL_YEAR = '2026 - 2027';

/** The worksheet the seeded class lives on. */
export const SEPTEMBER_SHEET = 'SEPTEMBER 2026';

/**
 * September 2026 school days: the template's grid has 24 merge-master columns and a
 * 31-day month has at most 23 Monday-Friday days, so every day fits without a sixth
 * week.
 */
export const SEPTEMBER_2026_DAYS = [
	1, 2, 3, 4, 7, 8, 9, 10, 11, 14, 15, 16, 17, 18, 21, 22, 23, 24, 25, 28, 29, 30
];

/** A row on the male block of the form, and one on the female block. */
export const MALE_ROW = 8;
export const SECOND_MALE_ROW = 9;
export const FEMALE_ROW = 30;
export const MALE_TOTAL_ROW = 29;

export const MALE_ONE = 'ALVARADO, ZYRON JAY  E.';
export const MALE_TWO = 'BAPTISMA, JONATHAN';
export const FEMALE_ONE = 'SALIMBOT, RAFA LATISHA';

// â”€â”€ a whole install, ready to diagnose â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/** The workbook fixture and the in-memory database one test is running against. */
export type Install = {
	driver: SqlDriver;
	fixture: TemplateFixture;
};

/**
 * Mount the real template on an in-memory file system and bind a fresh in-memory
 * database to the app's driver seam. Call from `beforeEach`.
 */
export async function mountInstall(): Promise<Install> {
	useTestWorkbookDir();
	const driver = new NodeSqlDriver();
	useDriver(driver);
	return { driver, fixture: await loadTemplate() };
}

/** Release the driver bound by {@link mountInstall}. Call from `afterEach`. */
export async function unmountInstall(install: Install): Promise<void> {
	useDriver(null);
	await install.driver.close();
}

/**
 * Build the referenced workbook the way a month workbook actually looks after the
 * pre-split calendar cycle has been through it: one visible month sheet carrying this
 * class's roster and three absences, and the other five renamed to `__SF2_HIDDEN_n`,
 * hidden and emptied of the template's sample marks.
 *
 * That shape matters and is not cosmetic. The hidden sheets are what a probe that skips
 * invisible worksheets would miss, and the rename is what stops them resolving to a month
 * of their own - which is why `headerMonthLabel` is reported rather than believed.
 */
export async function septemberWorkbook(
	fixture: TemplateFixture,
	mutate?: (workbook: ExcelJS.Workbook, september: ExcelJS.Worksheet) => void
): Promise<TemplateFixture> {
	const workbook = await fixture.open();
	const september = findSheet(workbook, 'SEPT. 2025');
	renameSheet(september, SEPTEMBER_SHEET);
	printRoster(september);
	printDayRow(september, SEPTEMBER_2026_DAYS);
	writeX(september, MALE_ROW, dayColumns(september)[0]!);
	writeX(september, SECOND_MALE_ROW, dayColumns(september)[2]!);
	writeX(september, MALE_ROW, dayColumns(september)[4]!);
	let hidden = 0;
	for (const sheet of workbook.worksheets) {
		if (sheet.name === SEPTEMBER_SHEET) continue;
		hidden += 1;
		hideSheet(sheet);
		renameSheet(sheet, `__SF2_HIDDEN_${hidden}`);
		// A month workbook carries marks for the month it is on. The template's sample
		// marks belong to the template, not to this class.
		clearMarks(sheet);
	}
	mutate?.(workbook, september);
	await fixture.roundTrip(workbook);
	return fixture;
}

/** The worksheet of an already-open workbook, or a loud failure. */
export function findSheet(workbook: { worksheets: Worksheet[] }, name: string): Worksheet {
	const sheet = workbook.worksheets.find((candidate) => candidate.name === name);
	if (sheet === undefined) throw new Error(`no worksheet named ${name}`);
	return sheet;
}

/** Put the seeded class's learners where the form keeps its roster. */
function printRoster(sheet: ExcelJS.Worksheet): void {
	// The template's own sample learners have to go: a name match against `students` must
	// be able to tell this class's three learners from the twelve the template ships.
	const maleEnd = MALE_TOTAL_ROW;
	const femaleEnd = SF2_FRESH_FEMALE_START_ROW + SF2_FRESH_FEMALE_SLOTS - 1;
	for (let row = SF2_FIRST_LEARNER_ROW; row <= femaleEnd; row += 1) {
		if (row === maleEnd) continue;
		if (cellText(sheet.getRow(row).getCell(3)).trim() === '') continue;
		setCellText(sheet, row, 3, '');
	}
	sheet.getRow(MALE_ROW).getCell(3).value = MALE_ONE;
	sheet.getRow(SECOND_MALE_ROW).getCell(3).value = MALE_TWO;
	sheet.getRow(FEMALE_ROW).getCell(3).value = FEMALE_ONE;
}

/**
 * Clear every `X` on a sheet.
 *
 * Only literal `X` cells are touched, and the day row is skipped. The template's total
 * rows carry shared formulas, and clearing a shared formula's master leaves its clones
 * unparseable; the day row holds the day numbers, and a day with nobody absent is still
 * a day.
 */
export function clearMarks(sheet: ExcelJS.Worksheet): void {
	for (let row = 1; row <= 80; row += 1) {
		if (row === SF2_DAY_ROW) continue;
		for (const column of dayColumns(sheet)) {
			const cell = sheet.getRow(row).getCell(column);
			if (cellText(cell).trim().toUpperCase() !== 'X') continue;
			cell.value = null;
		}
	}
}

export { loadTemplate, type TemplateFixture };

/**
 * Write a second workbook into the file system already bound by {@link loadTemplate}.
 *
 * `loadTemplate` rebinds the seam to a fresh `MemoryFileSystem`, so a test that needs
 * two files has to add the second one here rather than load the template twice. The
 * real install is exactly this shape: a referenced workbook and older working copies
 * sitting beside it.
 */
export async function addWorkbook(
	fixture: TemplateFixture,
	path: string,
	mutate: (workbook: ExcelJS.Workbook) => void
): Promise<void> {
	const workbook = await openWorkbook(fixture.path);
	mutate(workbook);
	await saveWorkbookAtomic(workbook, path);
}

/** Point the workbook-folder accessor at the in-memory folder for the whole suite. */
export function useTestWorkbookDir(): void {
	useSf2WorkbookDir(WORKBOOK_DIR);
}

/** The day-grid columns a mark can be addressed in, as numbers. */
export function dayColumns(sheet: Worksheet): number[] {
	return writableDayColumns(sheet).map(columnNumber);
}

/**
 * Print `days` into row 6, one per merge-master column, clearing the row first.
 *
 * Clearing matters: the template ships a full month of day numbers, and a test that
 * overwrote the first three slots would otherwise still find the other eighteen and
 * prove nothing.
 */
export function printDayRow(sheet: Worksheet, days: readonly number[]): number[] {
	const columns = dayColumns(sheet);
	for (const column of columns) setCellText(sheet, SF2_DAY_ROW, column, '');
	days.forEach((day, position) => {
		const column = columns[position];
		if (column === undefined) throw new Error(`the grid holds only ${columns.length} days`);
		setCellText(sheet, SF2_DAY_ROW, column, String(day));
	});
	return columns;
}

/** Clear row 6, so the sheet carries marks that cannot be resolved to a date. */
export function clearDayRow(sheet: Worksheet): void {
	for (const column of dayColumns(sheet)) setCellText(sheet, SF2_DAY_ROW, column, '');
}

export function writeX(sheet: Worksheet, row: number, column: number): void {
	setCellText(sheet, row, column, 'X');
}

/**
 * Put a day number into row 6 as a formula with a cached result.
 *
 * The ExcelJS trap this exists for: `cell.text` renders a cached `0` as blank and
 * throws outright on a merge cell whose master holds no value. A reader that trusts it
 * either loses a day column or falls over, and a lost day column is a day the teacher
 * cannot record an absence on. `cellText()` reads `cell.model.result` instead.
 */
export function writeDayFormula(
	sheet: Worksheet,
	column: number,
	formula: string,
	result: number
): void {
	sheet.getRow(SF2_DAY_ROW).getCell(column).value = { formula, result };
}

/** Hide a worksheet the way the pre-split calendar cycle leaves eleven of twelve. */
export function hideSheet(sheet: Worksheet): void {
	sheet.state = 'hidden';
}

/** Rename a worksheet, e.g. to the `__SF2_HIDDEN_n` the old cycle wrote. */
export function renameSheet(sheet: Worksheet, name: string): void {
	sheet.name = name;
}

// The database half of the fixture lives beside this file; re-exported so a test that
// wants "a whole install" needs one import, and so the class, learner and worksheet
// constants both halves agree on are declared exactly once.
export {
	DEFAULT_ABSENCES,
	createSchema,
	seedInstall,
	snapshotTables,
	unixSecondsOf,
	localDateOf,
	type SeedAbsence,
	type SeedOptions
} from './install-schema';
