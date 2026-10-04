/**
 * The lists that keep a verdict honest — the tail of
 * `src-tauri/src/sf2/diagnose/mod.rs`.
 *
 * Two lists, one job: a month the diagnostic could not place must still be visible.
 *
 * - {@link unplacedSheets} is every worksheet holding `X` cells that no month
 *   consumed. A worksheet nobody could place is not a worksheet with nothing on it,
 *   so a "the database is complete" answer is only honest while this is empty.
 * - {@link workbookReports} is one row per file examined, including the ones that would
 *   not open. A month is only `ExcelUnavailable` when *no* readable file names it, and
 *   the user needs to know which files were not read before they trust the rest.
 */

import { rawMarkAddress } from './compare';
import { markCount, nameAt, type RawWorkbook, type RawSheet } from './probe';
import type { UnplacedSheet, WorkbookFileReport } from './model';

/** One workbook's probe, successful or not. */
export interface WorkbookProbe {
	path: string;
	workbook?: RawWorkbook;
	readError?: string;
}

export function probeSheets(probe: WorkbookProbe): RawSheet[] {
	return probe.workbook?.sheets ?? [];
}

/** Stable identity for the `(file, worksheet)` pair a month was measured on. */
export function consumedKey(path: string, sheetName: string): string {
	return `${path} ${sheetName}`;
}

/** Every worksheet that holds marks and was not used for a month. */
export function unplacedSheets(
	probes: readonly WorkbookProbe[],
	consumed: ReadonlySet<string>
): UnplacedSheet[] {
	const unplaced: UnplacedSheet[] = [];
	for (const probe of probes) {
		for (const sheet of probeSheets(probe)) {
			if (consumed.has(consumedKey(probe.path, sheet.sheetName)) || markCount(sheet) === 0) {
				continue;
			}
			unplaced.push({
				workbookPath: probe.path,
				sheetName: sheet.sheetName,
				visible: sheet.visible,
				unrowedXCount: markCount(sheet),
				reason: unplacedReason(sheet),
				marks: sheet.marks.map((mark) => ({
					studentName: nameAt(sheet, mark.rowIndex) ?? '(no name in this row)',
					date: '(no day grid on this sheet)',
					sheetName: sheet.sheetName,
					cellAddress: rawMarkAddress(mark)
				}))
			});
		}
	}
	return unplaced;
}

/** Why a worksheet holding marks could not be tied to a month. */
function unplacedReason(sheet: RawSheet): string {
	const marks = markCount(sheet);
	if (sheet.monthFromName === undefined && sheet.dayNumbers.length === 0) {
		return (
			`Worksheet '${sheet.sheetName}' carries ${marks} X cell(s) but its name says no month ` +
			'and its day-number row is empty, so no cell on it can be resolved to a date. Its ' +
			'learner rows also belong to a roster the database does not hold. These cells were ' +
			'not compared with anything.'
		);
	}
	if (sheet.dayNumbers.length === 0) {
		return (
			`Worksheet '${sheet.sheetName}' carries ${marks} X cell(s) but its day-number row is ` +
			'empty, so no cell on it can be resolved to a date.'
		);
	}
	return (
		`Worksheet '${sheet.sheetName}' was not used for a month, so its ${marks} X cell(s) were ` +
		'not compared.'
	);
}

/** One row per candidate workbook. */
export function workbookReports(
	probes: readonly WorkbookProbe[],
	referenced: string | undefined
): WorkbookFileReport[] {
	return probes.map((probe) => {
		const sheets = probeSheets(probe);
		return {
			path: probe.path,
			isReferencedByDatabase: referenced !== undefined && referenced === probe.path,
			sheetCount: sheets.length,
			monthSheetsMeasured: sheets.filter(
				(sheet) => sheet.monthFromName !== undefined && sheet.dayNumbers.length > 0
			).length,
			totalXCount: sheets.reduce((sum, sheet) => sum + markCount(sheet), 0),
			readError: probe.readError
		};
	});
}

/**
 * Every workbook that would not open, one line each, or `undefined` when they all
 * opened.
 */
export function excelFailures(probes: readonly WorkbookProbe[]): string | undefined {
	const failures = probes
		.filter((probe) => probe.readError !== undefined)
		.map((probe) => `${probe.path}: ${probe.readError as string}`);
	return failures.length === 0 ? undefined : failures.join(' | ');
}

/** How many candidates failed to open. */
export function unreadableCount(probes: readonly WorkbookProbe[]): number {
	return probes.filter((probe) => probe.readError !== undefined).length;
}
