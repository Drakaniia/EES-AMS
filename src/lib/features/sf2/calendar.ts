/**
 * The SF2 calendar: month arithmetic, the first-attendance-day rules, and the
 * date-header writer.
 *
 * Ports `src-tauri/src/sf2/calendar/mod.rs` and the date half of
 * `excel_com/calendar.rs`. The pure rules live here; the writer takes an
 * `ExcelJS.Workbook` instead of a COM worksheet, because there is no Excel process
 * to drive (migration spec D2).
 *
 * ## Two different year rules, on purpose
 *
 * {@link sf2ReportYear} wraps the school year at **June**, because that is what
 * the legacy `sf2_templates` rows and the workbook's own sheet names were written
 * with. {@link reportYearForSchoolMonth} wraps at **September**, which is the
 * Philippine school year and what the v22 backfill used in SQL. They disagree for
 * June, July and August, and keeping both is deliberate: changing either one
 * silently re-dates rows that are already on disk.
 */

import ExcelJS from 'exceljs';
import type { Workbook, Worksheet } from 'exceljs';
import { internal, invalidInput } from '$lib/db';
import {
	SF2_ATTENDANCE_FIRST_COLUMN,
	SF2_ATTENDANCE_LAST_COLUMN,
	SF2_DAY_ROW,
	SF2_FRESH_FEMALE_TOTAL_ROW,
	SF2_FRESH_MALE_TOTAL_ROW,
	SF2_WEEKDAY_ROW
} from '$lib/features/excel/constants';
import { readLearnerRows } from '$lib/features/excel/roster';
import {
	activateSheet,
	cellText,
	getCellText,
	monthName,
	monthNumber,
	sf2MonthlySheets,
	writableCell
} from '$lib/features/excel/workbook';
import { addDays, lastDayOfMonth, naiveDate, parseIsoDate } from './first-school-day';

export { reportYearForSchoolMonth } from './first-school-day';

/**
 * The SF2 workbook, as the pure rules above see it.
 *
 * These are the Rust `sf2::models` records, minus the two fields that only ever
 * described the COM object it was read through (`file_format`, `has_vb_project`).
 * `$lib/features/excel/types` declares a differently-shaped `Sf2WorkbookAnalysis`
 * for the workbook *inventory*; this one is the business view — learners, dates
 * and the eight metadata fields — and the two are deliberately not merged.
 */

/** One learner row as a workbook stores it. */
export type Sf2WorkbookLearner = {
	rowIndex: number;
	name: string;
	/** `MALE` / `FEMALE`, decided by which TOTAL divider the row sits above. */
	genderBlock?: string;
	/**
	 * The DepEd learner ID, when the sheet carries one — which the bundled template
	 * does not, its learner-ID cell being merged into the `No.` cell.
	 */
	sf2LearnerId?: string;
};

/** One day column of a month sheet, resolved to a real date. */
export type Sf2WorkbookDate = {
	sheetName: string;
	/** `YYYY-MM-DD`. */
	date: string;
	columnLetter: string;
	/** 1-based, matching Excel. */
	columnIndex: number;
};

/** One worksheet of the workbook. */
type Sf2WorkbookSheet = {
	name: string;
	usedRange: string;
};

export type Sf2WorkbookAnalysis = {
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	learners: Sf2WorkbookLearner[];
	dates: Sf2WorkbookDate[];
	sheets: Sf2WorkbookSheet[];
};

/**
 * The eight header fields plus what the calendar writer needs.
 *
 * `configureCalendar` and `firstSchoolDay` are the two fields `$lib/features/excel`
 * has no home for: they decide whether the date header is written at all and from
 * which day, and a metadata block that cannot say "do not touch my calendar" is a
 * metadata block that rewrites a teacher's grid on every save.
 */
export type Sf2TemplateMetadata = {
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	configureCalendar: boolean;
	/** Absent means "not derived yet", never day zero. */
	firstSchoolDay?: number;
};

// ── Months and years ─────────────────────────────────────────────────────────

/** The month (1-12) a name represents, or `undefined` when it names none. */
export function sf2MonthNumber(name: string): number | undefined {
	const month = monthNumber(name);
	return month === 0 ? undefined : month;
}

/** The full uppercase English name of a month, or `''` when out of range. */
export function sf2MonthName(month: number): string {
	return monthName(month);
}

/**
 * The calendar year a report month falls in, wrapping the school year at June.
 *
 * Rust read only the *first* four-digit year of the label and ignored a missing
 * second one, so `2026` alone resolves; that is kept because the legacy
 * `sf2_templates` rows and the sheet names it is compared against were both
 * written this way. {@link reportYearForSchoolMonth} is the September-wrapping
 * rule the per-month model uses, and the two differ for June..August.
 */
export function sf2ReportYear(schoolYear: string, month: number, fallbackYear?: number): number {
	const startYear = schoolYear
		.split(/[^0-9]/)
		.find((part) => part.length === 4 && part.startsWith('20'));
	if (startYear === undefined) return fallbackYear ?? new Date().getFullYear();
	return month >= 6 ? Number(startYear) : Number(startYear) + 1;
}

/** The one worksheet a month lives on inside the one workbook: `SEPTEMBER 2026`. */
export function monthSheetName(reportMonth: number, reportYear: number): string {
	return `${monthName(reportMonth)} ${reportYear}`;
}

/** Whether the date belongs to this year and month. */
function isInMonth(date: Date, year: number, month: number): boolean {
	return date.getUTCFullYear() === year && date.getUTCMonth() + 1 === month;
}

// ── First attendance day ─────────────────────────────────────────────────────

/** The calendar year a report month falls in, from the first year in the label. */
function reportMonthNumber(reportMonth: string): number {
	const month = sf2MonthNumber(reportMonth);
	if (month === undefined) throw invalidInput('Report Month must be a valid month name');
	return month;
}

/**
 * Reject a first attendance day the month cannot hold.
 *
 * Two failures, two messages: a day the month does not have, and a Saturday or
 * Sunday, which no SF2 column can record an absence on.
 */
export function validateFirstSchoolDay(day: number, reportMonth: string, schoolYear: string): void {
	const month = reportMonthNumber(reportMonth);
	const year = sf2ReportYear(schoolYear, month);
	const lastDay = lastDayOfMonth(year, month);
	if (naiveDate(year, month, day) === undefined) {
		throw invalidInput(
			`First attendance day must be between 1 and ${lastDay} for this report month`
		);
	}
	if (!isSchoolDayOf(year, month, day)) {
		throw invalidInput('First attendance day must be a Monday-Friday school day');
	}
}

function isSchoolDayOf(year: number, month: number, day: number): boolean {
	const date = naiveDate(year, month, day);
	return date !== undefined && date.getUTCDay() >= 1 && date.getUTCDay() <= 5;
}

/** The first Monday-Friday of the report month, whether or not any date was seen. */
export function defaultSf2FirstSchoolDay(reportMonth: string, schoolYear: string): number {
	const month = reportMonthNumber(reportMonth);
	const year = sf2ReportYear(schoolYear, month);
	const lastDay = lastDayOfMonth(year, month);

	for (let day = 1; day <= lastDay; day += 1) {
		if (isSchoolDayOf(year, month, day)) return day;
	}
	throw internal('failed to find a Monday-Friday attendance day for this report month');
}

/**
 * The first attendance day of the report month, preferring what the workbook
 * already recorded.
 *
 * The dates are scanned in ascending order and the first one that passes
 * {@link validateFirstSchoolDay} wins, so a workbook whose earliest column is a
 * weekend (or holds a stale date from another month) still lands on a real school
 * day instead of failing the import.
 */
export function firstSchoolDayForReportMonth(
	reportMonth: string,
	schoolYear: string,
	dates: Iterable<string>
): number {
	const month = reportMonthNumber(reportMonth);
	const year = sf2ReportYear(schoolYear, month);
	const detectedDays = [
		...new Set(
			[...dates]
				.map((date) => parseIsoDate(date))
				.filter((date): date is Date => date !== undefined && date.getUTCMonth() + 1 === month)
				.map((date) => date.getUTCDate())
		)
	].sort((left, right) => left - right);

	// A day taken from a date in this month is already in range, so only the
	// weekday can reject it.
	for (const day of detectedDays) {
		if (isSchoolDayOf(year, month, day)) return day;
	}
	return defaultSf2FirstSchoolDay(reportMonth, schoolYear);
}

/**
 * Confirm the workbook's own dates start where the metadata says they should.
 *
 * The check is that the *earliest* detected date in the report month is the
 * expected first day, not that the set matches exactly: a teacher who added a
 * column by hand has still configured the calendar correctly.
 */
export function validateConfiguredCalendar(
	analysis: Sf2WorkbookAnalysis,
	metadata: Sf2TemplateMetadata
): void {
	if (!metadata.configureCalendar) return;
	if (metadata.firstSchoolDay === undefined) {
		throw invalidInput('First attendance day is required for SF2 templates');
	}

	const month = reportMonthNumber(metadata.reportMonth);
	const year = sf2ReportYear(metadata.schoolYear, month);
	const detectedDays = analysis.dates
		.map((mapping) => parseIsoDate(mapping.date))
		.filter((date): date is Date => date !== undefined && isInMonth(date, year, month))
		.map((date) => date.getUTCDate());
	if (detectedDays.length === 0) {
		throw internal('SF2 calendar was not configured correctly: no attendance dates were detected');
	}

	const actualDay = Math.min(...detectedDays);
	if (actualDay !== metadata.firstSchoolDay) {
		throw internal(
			`SF2 calendar was not configured correctly: expected first attendance day ${metadata.firstSchoolDay}, but the workbook starts at day ${actualDay}`
		);
	}
}

// ── The day grid ─────────────────────────────────────────────────────────────

/** One labelled day cell of the form. */
export type Sf2DaySlot = {
	/** 1-based Excel column of the day cell. */
	column: number;
	/** 0-based week, counting from the first week the month touches. */
	weekIndex: number;
	/** 0 = Monday .. 4 = Friday. */
	weekdayIndex: number;
};

/**
 * Parse a weekday label from row 7 into a 0-based index (0=Mon..4=Fri).
 *
 * Handles the common DepEd SF2 formats: "M", "MON", "MONDAY", "T", "TUE", "TH".
 */
export function parseWeekdayLabel(label: string): number | undefined {
	const upper = label.trim().toUpperCase();
	switch (upper[0]) {
		case 'M':
			return 0;
		case 'T':
			// "TH" (Thursday) vs "T" (Tuesday)
			return upper.startsWith('TH') ? 3 : 1;
		case 'W':
			return 2;
		case 'F':
			return 4;
		default:
			return undefined;
	}
}

/**
 * The labelled day cells of a sheet, read off its own weekday header (row 7).
 *
 * The template merges column pairs (`F7:G7` = Mon), and only the merge master holds
 * the label — which is why ExcelJS's `cellText` cannot be used to read the row: it
 * deliberately answers for a merge slave with its master's text, and counting each
 * half of a pair as a day cell would give 33 slots instead of 25 and shift every
 * week after the first. A non-weekday label (`ABSENT`, `PRESENT`) is skipped too.
 */
export function sf2WeekdaySlots(sheet: Worksheet): Sf2DaySlot[] {
	const slots: Sf2DaySlot[] = [];
	for (
		let column = SF2_ATTENDANCE_FIRST_COLUMN;
		column <= SF2_ATTENDANCE_LAST_COLUMN;
		column += 1
	) {
		const cell = sheet.getRow(SF2_WEEKDAY_ROW).getCell(column);
		if (cell.type === ExcelJS.ValueType.Merge) continue;
		const label = cellText(cell);
		if (label.trim() === '') continue;
		const weekdayIndex = parseWeekdayLabel(label);
		if (weekdayIndex === undefined) continue;
		slots.push({ column, weekIndex: Math.floor(slots.length / 5), weekdayIndex });
	}
	return slots;
}

/** 0 = Monday .. 4 = Friday; `undefined` for a weekend. */
function weekdayIndex(date: Date): number | undefined {
	const index = (date.getUTCDay() + 6) % 7;
	return index <= 4 ? index : undefined;
}

/**
 * The Monday of the week `date` falls in, whether or not `date` is a school day.
 *
 * {@link dayNumbersForSlots} counts school weeks *from* this date, so it has to be
 * a Monday even when the first day of the month is not one. A month that opens on a
 * weekend - November 2026 opens on a Sunday, say - has no weekday to step back
 * from, and taking the previous school day first is what keeps that month's grid
 * from coming out **empty**: every one of its day columns would otherwise be blank,
 * and a school term's worth of attendance with nowhere to record it.
 */
function mondayAnchorFor(date: Date): Date {
	const previousSchoolDay =
		date.getUTCDay() === 6 ? addDays(date, -1) : date.getUTCDay() === 0 ? addDays(date, -2) : date;
	return addDays(previousSchoolDay, -((previousSchoolDay.getUTCDay() + 6) % 7));
}

/**
 * Which week of the form a date falls in, counting from the Monday on or before
 * the first attendance day.
 *
 * A date before the anchor floors to a negative week, which no slot can match — the
 * anchor is the first attendance day itself or earlier, so this cannot happen in
 * practice, and a wrong day is far worse than no day.
 */
function weekNumber(date: Date, mondayAnchor: Date): number {
	return Math.floor((date.getTime() - mondayAnchor.getTime()) / (7 * 86_400_000));
}

/**
 * The day number that belongs in each slot, or `undefined` for a slot this month
 * has no school day for.
 *
 * The same pure layout the month-file builder uses, so a date-header write here and
 * a `sf2_month_date_mappings` row there cannot disagree about which day sits in
 * which column. Two copies of this arithmetic is how a day's absence ends up in a
 * column the AMOUNT formulas do not count.
 */
export function dayNumbersForSlots(
	reportYear: number,
	reportMonth: number,
	firstSchoolDay: number,
	slots: readonly Sf2DaySlot[]
): { column: number; day?: number }[] {
	const lastDay = lastDayOfMonth(reportYear, reportMonth);
	const firstDate = naiveDate(reportYear, reportMonth, firstSchoolDay);
	const anchor = firstDate === undefined ? undefined : mondayAnchorFor(firstDate);

	return slots.map((slot) => {
		if (anchor === undefined) return { column: slot.column };
		for (let day = firstSchoolDay; day <= lastDay; day += 1) {
			const date = naiveDate(reportYear, reportMonth, day);
			if (date === undefined) continue;
			if (weekdayIndex(date) === slot.weekdayIndex && weekNumber(date, anchor) === slot.weekIndex) {
				return { column: slot.column, day };
			}
		}
		return { column: slot.column };
	});
}

/**
 * The school days of a month that the form has no day column for.
 *
 * The DepEd SF2 grid is **five weeks of Monday..Friday - 25 labelled day cells** -
 * and the ABSENT/PRESENT block begins in the very next column (`AM`), so a sixth
 * week has nowhere to go. That is 25 slots for a month that can hold at most **23**
 * Monday-Friday days (a 31-day month starting on a Monday is the maximum), so
 * nothing is ever dropped and this returns an empty array for every real month.
 *
 * It is still computed by every writer, because "it cannot happen" is the weakest
 * possible guard for the one failure that costs a user a term of marks: a school
 * day silently missing from the grid is a day the user cannot record an absence on.
 */
export function daysWithoutASlot(
	reportYear: number,
	reportMonth: number,
	firstSchoolDay: number,
	slots: readonly Sf2DaySlot[]
): number[] {
	const placed = new Set(
		dayNumbersForSlots(reportYear, reportMonth, firstSchoolDay, slots)
			.map((entry) => entry.day)
			.filter((day): day is number => day !== undefined)
	);
	const lastDay = lastDayOfMonth(reportYear, reportMonth);

	const dropped: number[] = [];
	for (let day = firstSchoolDay; day <= lastDay; day += 1) {
		if (placed.has(day)) continue;
		if (isSchoolDayOf(reportYear, reportMonth, day)) dropped.push(day);
	}
	return dropped;
}

// ── Writing the date header ──────────────────────────────────────────────────

/** What {@link configureSf2Calendar} did, for the caller to report on. */
type Sf2CalendarResult = {
	sheetName: string;
	/** School days the DepEd form has no column for. Empty for every real month. */
	droppedSchoolDays: number[];
};

/** How well populated a monthly sheet is; compared field by field, highest wins. */
type Sf2SheetQuality = {
	totalDayCells: number;
	learnerCount: number;
	maleCount: number;
	femaleCount: number;
};

function compareQuality(left: Sf2SheetQuality, right: Sf2SheetQuality): number {
	return (
		left.totalDayCells - right.totalDayCells ||
		left.learnerCount - right.learnerCount ||
		left.maleCount - right.maleCount ||
		left.femaleCount - right.femaleCount
	);
}

/**
 * Assess a sheet on learner and day data.
 *
 * The TOTAL rows' day cells come first in the comparison: a sheet whose totals
 * were never written has no attendance grid at all, however many names it holds.
 */
function sf2SheetQuality(sheet: Worksheet): Sf2SheetQuality {
	const learners = readLearnerRows(sheet);
	let totalDayCells = 0;
	for (const row of [SF2_FRESH_MALE_TOTAL_ROW, SF2_FRESH_FEMALE_TOTAL_ROW]) {
		for (
			let column = SF2_ATTENDANCE_FIRST_COLUMN;
			column <= SF2_ATTENDANCE_LAST_COLUMN;
			column += 1
		) {
			if (getCellText(sheet, row, column).trim() !== '') totalDayCells += 1;
		}
	}
	return {
		totalDayCells,
		learnerCount: learners.length,
		maleCount: learners.filter((learner) => learner.gender === 'M').length,
		femaleCount: learners.filter((learner) => learner.gender === 'F').length
	};
}

/** The most populated monthly sheet, which is the one a fallback write targets. */
export function bestSf2MonthlySheet(sheets: readonly Worksheet[]): Worksheet | undefined {
	let best: Worksheet | undefined;
	let bestQuality: Sf2SheetQuality | undefined;
	for (const sheet of sheets) {
		const quality = sf2SheetQuality(sheet);
		if (bestQuality === undefined || compareQuality(quality, bestQuality) > 0) {
			best = sheet;
			bestQuality = quality;
		}
	}
	return best;
}

/**
 * Rename a sheet to `baseName`, truncated to the 31 characters Excel allows.
 *
 * No uniquifying counter: the caller only renames when no sheet already carries the
 * name (it renames the exact match, or a fallback chosen *because* the name is
 * absent), so there is never a duplicate to step around.
 */
function renameSheet(sheet: Worksheet, baseName: string): void {
	sheet.name = baseName.slice(0, 31);
}

/**
 * Write the month's day numbers across the merged weekday pairs of row 6.
 *
 * Every slot is written, including the empty ones: a sheet carried over from
 * another month still holds that month's numbers, and a stale day number in a
 * column is read as a school day that does not exist.
 *
 * Each write lands on the merge master only, so a weekday pair's second column
 * can never clobber the first (the F7:G7-style overwrite).
 */
export function setSf2MonthDates(
	sheet: Worksheet,
	reportYear: number,
	reportMonth: number,
	firstSchoolDay: number
): number[] {
	const slots = sf2WeekdaySlots(sheet);
	if (slots.length === 0) return [];

	const lastDay = lastDayOfMonth(reportYear, reportMonth);
	if (firstSchoolDay < 1 || firstSchoolDay > lastDay) {
		throw invalidInput(
			`First attendance day must be between 1 and ${lastDay} for this report month`
		);
	}
	if (!isSchoolDayOf(reportYear, reportMonth, firstSchoolDay)) {
		throw invalidInput('First attendance day must be a Monday-Friday school day');
	}

	for (const { column, day } of dayNumbersForSlots(
		reportYear,
		reportMonth,
		firstSchoolDay,
		slots
	)) {
		// The day cell is merged with its weekend half, so the write lands on the
		// merge master and has to be pushed left to read as a day rather than a
		// right-aligned number in a wide cell.
		const cell = writableCell(sheet.getRow(SF2_DAY_ROW).getCell(column));
		cell.value = day === undefined ? null : String(day);
		cell.alignment = { ...cell.alignment, horizontal: 'left', indent: 0 };
	}

	return daysWithoutASlot(reportYear, reportMonth, firstSchoolDay, slots);
}

/**
 * Write the month's day numbers and make its worksheet the landing tab.
 *
 * The target sheet is the one named `"{MONTH} {year}"`; when the workbook has no
 * such tab — a single-sheet workbook, or a school's own variant naming — the most
 * populated monthly sheet is used instead, so the header lands somewhere real
 * rather than being silently dropped.
 */
export function configureSf2Calendar(
	workbook: Workbook,
	metadata: Sf2TemplateMetadata
): Sf2CalendarResult {
	const reportMonth = reportMonthNumber(metadata.reportMonth);
	const reportYear = sf2ReportYear(metadata.schoolYear, reportMonth);
	const targetName = monthSheetName(reportMonth, reportYear);

	const monthlySheets = sf2MonthlySheets(workbook);
	const sheet =
		monthlySheets.find((candidate) => candidate.name === targetName) ??
		bestSf2MonthlySheet(monthlySheets) ??
		monthlySheets[0];
	if (sheet === undefined) throw internal('SF2 workbook has no monthly worksheet to configure');

	sheet.state = 'visible';
	renameSheet(sheet, targetName);
	const droppedSchoolDays = setSf2MonthDates(
		sheet,
		reportYear,
		reportMonth,
		metadata.firstSchoolDay ?? 1
	);
	activateSheet(workbook, sheet.name);

	return { sheetName: sheet.name, droppedSchoolDays };
}
