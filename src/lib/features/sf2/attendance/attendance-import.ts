/**
 * Reading a month's `X` marks back out of the workbook.
 *
 * ## Why this exists
 *
 * The workbook is the school's official record, so it is the only surviving copy
 * of a day's absences when the app's database has been reset. Names and SF2
 * details are rebuilt from the workbook on import, but attendance marks were
 * historically write-only. This closes that gap.
 *
 * ## It is additive only
 *
 * An `X` becomes an `absent` event unless the database already records that
 * learner absent for that day. Marks in the database but absent from the workbook
 * are never removed here — a workbook is a recovery source, not the authority on
 * what the app recorded. So re-running is a safe no-op.
 */

import { sf2MonthName } from '$lib/features/sf2/calendar';
import { resolvedSheetName } from '$lib/features/sf2/month/templates';
import type { Sf2MonthDateMappingRecord } from '$lib/features/sf2/month/templates';
import type { Sf2MonthStudentMapping } from '$lib/features/sf2/month/students';
import { SF2_ABSENT_MARK } from '$lib/features/sf2/logic';
import type { Sf2AttendanceImportOutcome } from '$lib/types';
import { hasAbsentEventForDay, setAttendanceEventForDay } from './attendance-events';

/** Recorded on every event this module writes, so the trail says "from the workbook". */
export const IMPORT_REASON = 'SF2 workbook import';

export type { Sf2AttendanceImportOutcome };

/**
 * Does this workbook cell text count as an absence?
 *
 * Compared against the same constant the writer uses, so a change to the mark
 * written to Excel automatically changes what is read back. Case and surrounding
 * whitespace are ignored because Excel is not the only thing that has written in
 * those cells.
 */
export function isAbsentMark(text: string): boolean {
	return text.trim().toUpperCase() === SF2_ABSENT_MARK;
}

/** One cell of the grid an import inspects. */
export type Sf2ScanCell = { sheetName: string; address: string };

/**
 * Every `(sheet, address)` pair the import inspects: one cell per mapped learner
 * row × mapped date column, de-duplicated and in a stable order.
 *
 * Row `0` is skipped — the "not linked to a workbook row" placeholder has no cell
 * in any sheet and must never be scanned.
 */
export function gridCellsToScan(
	studentMappings: readonly Sf2MonthStudentMapping[],
	dateMappings: readonly Sf2MonthDateMappingRecord[]
): Sf2ScanCell[] {
	const seen = new Set<string>();
	const cells: Sf2ScanCell[] = [];
	for (const date of dateMappings) {
		const sheetName = resolvedSheetName(date);
		for (const mapping of studentMappings) {
			if (mapping.rowIndex <= 0) continue;
			const address = `${date.columnLetter}${mapping.rowIndex}`;
			const key = `${sheetName}!${address}`;
			if (seen.has(key)) continue;
			seen.add(key);
			cells.push({ sheetName, address });
		}
	}
	return cells;
}

/** An import that had nothing to scan: no roster, or no mapped days. */
export function emptyImportOutcome(
	classId: string,
	reportMonth: string,
	mappedDates: number
): Sf2AttendanceImportOutcome {
	return {
		classId,
		reportMonth,
		scannedCells: 0,
		imported: 0,
		alreadyRecorded: 0,
		datesWithMarks: 0,
		mappedRows: 0,
		mappedDates
	};
}

/** The canonical uppercase month a `YYYY-MM-DD` falls in, or `''`. */
function monthNameOf(date: string): string {
	const match = /^\d{4}-(\d{2})-\d{2}$/.exec(date.trim());
	return match === null ? '' : sf2MonthName(Number(match[1]));
}

/**
 * Record the absences a scan found.
 *
 * Separated from the workbook read so the rule — additive, idempotent, one event
 * per learner-day — can be tested without a workbook, and so the caller controls
 * when the file is opened.
 *
 * `textFor` returns `undefined` for a cell it could not read, which is neither an
 * absence nor a blank: such a cell is skipped rather than treated as present,
 * because that would silently drop a mark.
 */
export async function importAbsentMarks(params: {
	classId: string;
	reportMonth: string;
	dayStart: string;
	roster: readonly Sf2MonthStudentMapping[];
	dates: readonly Sf2MonthDateMappingRecord[];
	textFor: (sheetName: string, address: string) => string | undefined;
}): Promise<Sf2AttendanceImportOutcome> {
	const { classId, reportMonth, dayStart, roster, dates, textFor } = params;
	const scanned = gridCellsToScan(roster, dates);
	let imported = 0;
	let alreadyRecorded = 0;
	const datesWithMarks = new Set<string>();

	for (const date of dates) {
		const sheetName = resolvedSheetName(date);
		for (const mapping of roster) {
			if (mapping.rowIndex <= 0) continue;
			const text = textFor(sheetName, `${date.columnLetter}${mapping.rowIndex}`);
			if (text === undefined || !isAbsentMark(text)) continue;

			if (await hasAbsentEventForDay(mapping.studentId, classId, date.date)) {
				alreadyRecorded += 1;
			} else {
				await setAttendanceEventForDay({
					studentId: mapping.studentId,
					classId,
					date: date.date,
					dayStart,
					eventType: 'absent',
					reason: IMPORT_REASON
				});
				imported += 1;
			}
			datesWithMarks.add(date.date);
		}
	}

	return {
		classId,
		reportMonth: dates[0] === undefined ? reportMonth : monthNameOf(dates[0].date),
		scannedCells: scanned.length,
		imported,
		alreadyRecorded,
		datesWithMarks: datesWithMarks.size,
		mappedRows: roster.length,
		mappedDates: dates.length
	};
}
