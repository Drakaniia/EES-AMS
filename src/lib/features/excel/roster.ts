/**
 * Learner rows: reading the roster, growing the roster area, hiding empty slots.
 *
 * Replaces `excel_com/learners.rs` and the roster half of
 * `excel_com/workbook_ops.rs`. The row arithmetic is the whole risk of this file:
 * one row off and the TOTAL rows, the `COUNTIF` ranges and the teacher's printed
 * report all shift together, silently.
 */

import type { Workbook, Worksheet } from 'exceljs';
import {
	SF2_DEPED_LEARNER_ID_COLUMN,
	SF2_FIRST_LEARNER_ROW,
	SF2_FRESH_FEMALE_TOTAL_ROW,
	SF2_FRESH_MALE_TOTAL_ROW,
	SF2_ITEM_NUMBER_COLUMN,
	SF2_NAME_COLUMN
} from './constants';
import { getCellText, materialiseSharedFormulas, sf2MonthlySheets, setCellText } from './workbook';
import type { Sf2LearnerRow } from './types';

const MALE_BLOCK = 'M';
const FEMALE_BLOCK = 'F';

/** `NAME (Last Name, First Name, Middle Name)` with runs of space collapsed. */
function normalizeLearnerName(name: string): string {
	return name.trim().split(/\s+/).join(' ').replace(/, /g, ',').trim().toUpperCase();
}

/**
 * Whether a cell in the NAME column holds a learner rather than a form label.
 *
 * The form's own rows - the column header, the TOTAL Per Day rows, the Combined
 * TOTAL row - all live in the same column, so this is what separates a learner
 * from the form.
 */
export function isLearnerName(value: string): boolean {
	const trimmed = value.trim();
	if (trimmed === '') return false;
	const normalized = normalizeLearnerName(trimmed);
	if (
		normalized === 'NAME (LAST NAME,FIRST NAME,MIDDLE NAME)' ||
		normalized.includes('TOTAL PER DAY') ||
		normalized.includes('COMBINED TOTAL') ||
		normalized.includes('<===') ||
		normalized.includes('===')
	) {
		return false;
	}
	return normalized.includes(',') && /[A-Za-z]/.test(normalized);
}

/**
 * The DepEd learner ID, when the sheet actually carries one.
 *
 * The bundled template merges `A8:B8`, so column 2 returns the *item number*
 * rather than an identity. Taking it at face value would invent a DepEd ID out of
 * the item number, so a value that equals the item number is rejected and the
 * learner keeps no ID.
 */
export function depedLearnerIdFromCells(
	learnerIdCell: string,
	itemNumber: string
): string | undefined {
	const candidate = learnerIdCell.trim();
	if (candidate === '') return undefined;
	if (candidate === itemNumber.trim()) return undefined;
	return candidate;
}

/** Which half of the roster a row sits in; `undefined` after FEMALE TOTAL. */
type GenderBlock = typeof MALE_BLOCK | typeof FEMALE_BLOCK | undefined;

/**
 * Every learner row of one sheet, with the gender block it sits in.
 *
 * The gender is read from position rather than from a column: the form has no
 * gender column, only the `MALE TOTAL` / `FEMALE TOTAL` divider rows, so a
 * learner is a boy or a girl according to which divider they appear above. A
 * row that is in neither block is not a learner.
 *
 * The scan starts at {@link SF2_FIRST_LEARNER_ROW} and stops once the FEMALE
 * TOTAL divider has passed. Rows 1-7 are the form's own title and date headers,
 * and rows below Combined TOTAL are footnotes; because both are merged they
 * answer from their top-left cell, which reads back as plausible comma-separated
 * text rather than as obviously-wrong text.
 */
export function readLearnerRows(sheet: Worksheet): Sf2LearnerRow[] {
	let block: GenderBlock = MALE_BLOCK;
	const learners: Sf2LearnerRow[] = [];

	for (let row = SF2_FIRST_LEARNER_ROW; row <= sheet.rowCount; row += 1) {
		const name = getCellText(sheet, row, SF2_NAME_COLUMN).trim();
		if (name === '') continue;

		// `FEMALE` is tested first because it contains `MALE`; testing MALE first
		// would read the FEMALE TOTAL divider as a MALE one and leave the whole
		// footnote block below it looking like girls.
		const upper = name.toUpperCase();
		if (upper.includes('FEMALE') && upper.includes('TOTAL')) {
			block = undefined;
			continue;
		}
		if (upper.includes('MALE') && upper.includes('TOTAL')) {
			block = FEMALE_BLOCK;
			continue;
		}
		if (!isLearnerName(name) || block === undefined) continue;

		const itemNumber = getCellText(sheet, row, SF2_ITEM_NUMBER_COLUMN);
		const learnerIdCell = getCellText(sheet, row, SF2_DEPED_LEARNER_ID_COLUMN);
		learners.push({
			row,
			name,
			gender: block,
			learnerId: depedLearnerIdFromCells(learnerIdCell, itemNumber)
		});
	}
	return learners;
}

/** Every learner row of every visible monthly sheet, keyed by sheet name. */
export function readRoster(workbook: Workbook): Map<string, Sf2LearnerRow[]> {
	const roster = new Map<string, Sf2LearnerRow[]>();
	for (const sheet of sf2MonthlySheets(workbook)) {
		roster.set(sheet.name, readLearnerRows(sheet));
	}
	return roster;
}

/**
 * Grow the roster area by inserting rows before the MALE TOTAL and FEMALE TOTAL
 * rows of every monthly sheet.
 *
 * `maleTotalRow` / `femaleTotalRow` are the *current* positions of the TOTAL
 * rows - pass `undefined` for a fresh template, which defaults to 29 and 49.
 * Incremental expansions of an already-expanded workbook must pass the actual
 * positions, or the inserts land on top of the TOTAL rows.
 *
 * ponytail: ExcelJS's `spliceRows` moves values, styles, heights and merges down
 * but does not rewrite formula text, where Excel's `EntireRow.Insert` would. That
 * is safe here only because the caller regenerates every TOTAL and learner
 * formula through `totalFormulaMarks` and `learnerAbsentPresentFormulaMarks`
 * with the new row numbers - which the Rust had to do anyway, to keep the
 * template's stale `AW5` and subtotal caches honest. Shared formulas are
 * materialised first, because moving a shared formula's master without its clones
 * leaves the workbook unserialisable.
 */
export function expandRosterRows(
	workbook: Workbook,
	extraMaleRows: number,
	extraFemaleRows: number,
	maleTotalRow = SF2_FRESH_MALE_TOTAL_ROW,
	femaleTotalRow = SF2_FRESH_FEMALE_TOTAL_ROW
): number {
	if (extraMaleRows <= 0 && extraFemaleRows <= 0) return 0;

	let touched = 0;
	for (const sheet of sf2MonthlySheets(workbook)) {
		materialiseSharedFormulas(sheet);
		if (extraMaleRows > 0) {
			for (let index = 0; index < extraMaleRows; index += 1) sheet.spliceRows(maleTotalRow, 0, []);
		}
		if (extraFemaleRows > 0) {
			// The female divider sits below everything the male insert pushed down.
			const femaleBase = femaleTotalRow + extraMaleRows;
			for (let index = 0; index < extraFemaleRows; index += 1) sheet.spliceRows(femaleBase, 0, []);
		}
		touched += 1;
	}
	return touched;
}

/**
 * Hide the learner slots that hold no data, so only rows with real students print.
 *
 * Male slots run `8 .. maleTotalRow - 1`; female slots run
 * `maleTotalRow + 1 .. femaleTotalRow - 1` - the FEMALE TOTAL row itself is
 * never a slot. `occupiedRows` is the set of rows the roster mapping claims.
 */
export function hideEmptyLearnerRows(
	workbook: Workbook,
	maleTotalRow: number,
	femaleTotalRow: number,
	occupiedRows: ReadonlySet<number>
): number {
	let touched = 0;
	for (const sheet of sf2MonthlySheets(workbook)) {
		for (const [first, last] of [
			[SF2_FIRST_LEARNER_ROW, maleTotalRow - 1],
			[maleTotalRow + 1, femaleTotalRow - 1]
		]) {
			for (let row = first; row <= last; row += 1) {
				sheet.getRow(row).hidden = !occupiedRows.has(row);
			}
		}
		touched += 1;
	}
	return touched;
}

/** The learner slots of one sheet, as `{ row, gender }`. */
export function learnerSlots(
	maleTotalRow: number,
	femaleTotalRow: number
): { row: number; gender: 'M' | 'F' }[] {
	const slots: { row: number; gender: 'M' | 'F' }[] = [];
	for (let row = SF2_FIRST_LEARNER_ROW; row < maleTotalRow; row += 1)
		slots.push({ row, gender: 'M' });
	for (let row = maleTotalRow + 1; row < femaleTotalRow; row += 1) slots.push({ row, gender: 'F' });
	return slots;
}

/** Write a learner's name into a slot, renumbering the item-number column. */
export function writeLearnerRow(
	sheet: Worksheet,
	row: number,
	name: string,
	itemNumber: number
): void {
	setCellText(sheet, row, SF2_ITEM_NUMBER_COLUMN, String(itemNumber));
	setCellText(sheet, row, SF2_NAME_COLUMN, name);
}
