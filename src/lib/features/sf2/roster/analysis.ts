import type { Workbook, Worksheet } from 'exceljs';
import {
	SF2_ATTENDANCE_FIRST_COLUMN,
	SF2_ATTENDANCE_LAST_COLUMN,
	SF2_DAY_ROW,
	SF2_FORM_TITLE,
	SF2_METADATA_CELLS
} from '$lib/features/excel/constants';
import { readLearnerRows } from '$lib/features/excel/roster';
import {
	cellText,
	columnLetter,
	getCellText,
	sf2MonthlySheets,
	yearFromSheetName
} from '$lib/features/excel/workbook';
import { bestSf2MonthlySheet, sf2MonthNumber, sf2ReportYear } from '../calendar';
import { naiveDate } from '../first-school-day';
import type { Sf2WorkbookAnalysis, Sf2WorkbookDate, Sf2WorkbookLearner } from '../calendar';

/**
 * Reading a workbook as the SF2 business view: the eight header fields, every
 * learner row, every day column, and the sheet inventory.
 *
 * This is the whole-file read that produces the `Sf2WorkbookAnalysis` shape
 * `$lib/features/sf2/calendar` declares. That shape is the business view rather than
 * the workbook inventory, so it belongs with the roster rules that consume it rather
 * than in `$lib/features/excel`.
 *
 * The day grid is read from the **writable** day columns only. The form merges
 * consecutive day columns into pairs, and a read of the right half of a pair returns
 * its master's text, so reading every column produces every day twice. A mapping is an
 * address a write lands on, so a duplicated one is not a duplicate row - it is two
 * writes fighting over one cell.
 */

/** One metadata cell's text, or `''`. Every SF2 header field lives in a merged cell. */
function metadataText(sheet: Worksheet, field: keyof typeof SF2_METADATA_CELLS): string {
	const { row, column } = SF2_METADATA_CELLS[field];
	return cellText(sheet.getRow(row).getCell(column)).trim();
}

/** The eight header fields, as the form holds them on one sheet. */
function readMetadata(
	sheet: Worksheet
): Omit<Sf2WorkbookAnalysis, 'learners' | 'dates' | 'sheets'> {
	const signature = metadataText(sheet, 'adviserSignature');
	return {
		schoolId: metadataText(sheet, 'schoolId'),
		schoolName: metadataText(sheet, 'schoolName'),
		schoolYear: metadataText(sheet, 'schoolYear'),
		reportMonth: metadataText(sheet, 'reportMonth'),
		gradeLevel: metadataText(sheet, 'gradeLevel'),
		section: metadataText(sheet, 'section'),
		// The signature cell is the one schools actually fill in; the printed-name
		// cell is what a workbook carrying only that one falls through to.
		adviserName: signature !== '' ? signature : metadataText(sheet, 'adviserPrintedName'),
		schoolHeadName: metadataText(sheet, 'schoolHeadPrintedName')
	};
}

/**
 * The day columns of one sheet, resolved against its own day-number row.
 *
 * A column with no day number in it is not a mapping: a sheet carried over from
 * another month still holds that month's numbers, and a stale day is read as a
 * school day that does not exist. A day the month cannot hold - 31 in April - is
 * skipped rather than normalised, because a date nobody attended is worse than a
 * column with no mapping.
 */
function readSheetDates(sheet: Worksheet, year: number, month: number): Sf2WorkbookDate[] {
	const dates: Sf2WorkbookDate[] = [];
	for (
		let column = SF2_ATTENDANCE_FIRST_COLUMN;
		column <= SF2_ATTENDANCE_LAST_COLUMN;
		column += 1
	) {
		if (sheet.getRow(SF2_DAY_ROW).getCell(column).isMerged) continue;
		const text = getCellText(sheet, SF2_DAY_ROW, column).trim();
		const day = Number.parseInt(text, 10);
		if (!/^\d{1,2}$/.test(text) || !(day >= 1 && day <= 31)) continue;
		if (naiveDate(year, month, day) === undefined) continue;
		dates.push({
			sheetName: sheet.name,
			date: `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`,
			columnLetter: columnLetter(column),
			columnIndex: column
		});
	}
	return dates;
}

/** Whether a worksheet carries the DepEd School Form 2 title. */
function carriesFormTitle(sheet: Worksheet): boolean {
	return cellText(sheet.getRow(1).getCell(1)).toUpperCase().includes(SF2_FORM_TITLE.toUpperCase());
}

function toLearnerRows(sheet: Worksheet | undefined): Sf2WorkbookLearner[] {
	if (sheet === undefined) return [];
	return readLearnerRows(sheet).map((learner) => ({
		rowIndex: learner.row,
		name: learner.name,
		genderBlock: learner.gender === 'M' ? 'MALE' : learner.gender === 'F' ? 'FEMALE' : undefined,
		sf2LearnerId: learner.learnerId
	}));
}

/**
 * Read an already-open workbook as the SF2 business view.
 *
 * Only visible monthly sheets contribute learners and dates, which is what stops the
 * bundled template's `COMPLETE DAYS` copy of the form - a full roster of names that
 * are not this teacher's students - from being read as a month.
 */
export function readWorkbookAnalysis(workbook: Workbook): Sf2WorkbookAnalysis {
	const sheets = workbook.worksheets.map((sheet) => ({
		name: sheet.name,
		usedRange: sheet.dimensions?.toString() ?? ''
	}));
	const monthly = sf2MonthlySheets(workbook);

	const dates: Sf2WorkbookDate[] = [];
	for (const sheet of monthly) {
		const month = sf2MonthNumber(sheet.name);
		const year = yearFromSheetName(sheet.name);
		if (month === undefined || year === 0) continue;
		dates.push(...readSheetDates(sheet, year, month));
	}

	// No sheet the app would write to. A school's own single-sheet variant still
	// describes itself in its header cells, so the block below reads them there.
	let metadata = monthly[0] === undefined ? undefined : readMetadata(monthly[0]);
	let learnerSheet = bestSf2MonthlySheet(monthly);

	if (metadata === undefined) {
		const fallback = workbook.worksheets.find(
			(sheet) => sheet.state === 'visible' && carriesFormTitle(sheet)
		);
		if (fallback !== undefined) {
			metadata = readMetadata(fallback);
			learnerSheet = fallback;
			// The sheet name carries no month here, so the dates come from the
			// header's own report month instead.
			const month = sf2MonthNumber(metadata.reportMonth) ?? 1;
			dates.push(...readSheetDates(fallback, sf2ReportYear(metadata.schoolYear, month), month));
		}
	}

	return {
		schoolId: '',
		schoolName: '',
		schoolYear: '',
		reportMonth: '',
		gradeLevel: '',
		section: '',
		adviserName: '',
		schoolHeadName: '',
		...metadata,
		learners: toLearnerRows(learnerSheet),
		dates,
		sheets
	};
}
