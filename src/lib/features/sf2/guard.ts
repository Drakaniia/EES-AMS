/**
 * The destructive-sync guard.
 *
 * One property, held by two independent layers on purpose:
 *
 * > **An X mark in a workbook can only be removed by the user explicitly
 * > marking that student present.**
 *
 * 1. **The permit.** A write path evaluates a {@link SyncPermit} and acts on
 *    {@link actionFor}. `Unmeasured` is the default: a missing file, an
 *    unreadable workbook, an empty mapping set — every one of them resolves to
 *    `Unmeasured`, never to `Proven`. **When in doubt, do not clear.**
 * 2. **The differential clear** (`attendance-marks.ts`). The cells a sync blanks
 *    are computed from the diff, so a caller that forgot the guard still cannot
 *    wipe the grid.
 *
 * ExcelJS replaces COM here: measuring is a cell-text read off an opened
 * workbook, and the read-only branch skips all writes (there is no COM
 * `ReadOnly:=True`; the guarantee is "the app never writes").
 */

import type { Workbook, Worksheet } from 'exceljs';
import { getSheet } from '$lib/features/excel/workbook';
import type { Sf2MonthDateMappingRecord } from '$lib/features/sf2/month/templates';
import { resolvedSheetName } from '$lib/features/sf2/month/templates';
import type { Sf2MonthStudentMapping } from '$lib/features/sf2/month/students';
import { gridCellKey, workbookXCells, type Sf2GridCell } from './attendance/attendance-marks';

/** `Unmeasured` when the month has no mapped attendance dates. */
export const NO_MAPPED_DATES =
	'This SF2 workbook has no mapped attendance dates, so the app cannot tell which workbook ' +
	'cells belong to which day. Nothing was cleared.';

/** `Unmeasured` when the month has no mapped learners. */
export const NO_MAPPED_LEARNERS =
	'This SF2 workbook has no learners mapped to workbook rows, so the app cannot tell which ' +
	'cells hold attendance. Nothing was cleared.';

/** `Unmeasured` when the workbook could not be read at all. */
export const WORKBOOK_NOT_READABLE =
	'The SF2 workbook could not be read, so the app cannot prove the database holds its marks. ' +
	'Nothing was cleared.';

/** May the write path clear this month's attendance grid? */
export type SyncPermit =
	| { kind: 'Proven'; dbCount: number; workbookCount: number }
	| {
			kind: 'Stale';
			dbCount: number;
			workbookCount: number;
			/** `(student, date)` for every workbook X the database cannot produce. */
			missing: [string, string][];
	  }
	| { kind: 'Unmeasured'; reason: string };

/** What a write path must do with a permit. */
export type SyncAction =
	| { kind: 'Rewrite'; dbCount: number; workbookCount: number }
	| {
			kind: 'ImportThenRecheck';
			dbCount: number;
			workbookCount: number;
			missing: [string, string][];
	  }
	| { kind: 'ReadOnly'; reason: string }
	| { kind: 'Aborted'; message: string };

/** No workbook cells forgiven: the pre-correction behaviour. */
const EMPTY_KEYS: ReadonlySet<string> = new Set();

/**
 * The whole permit decision, with no workbook and no database in sight.
 *
 * `workbookX` is `undefined` when the workbook could not be measured at all —
 * the single input that produces `Unmeasured`.
 *
 * `Proven` requires **both**: `db_count >= workbook_count` AND every workbook
 * X is one the database can produce. Counts alone do not prove *which* absences
 * are held.
 *
 * `forgivenKeys` are workbook cells the database explicitly marks present (a
 * teacher's correction, never present-by-default silence): they are stale marks
 * the rewrite may clear, not marks the import must resurrect.
 */
export function decide(
	dbXCells: readonly Sf2GridCell[],
	workbookX: readonly Sf2GridCell[] | undefined,
	labels: ReadonlyMap<string, [string, string]>,
	reason: string,
	forgivenKeys: ReadonlySet<string> = EMPTY_KEYS
): SyncPermit {
	if (workbookX === undefined) return { kind: 'Unmeasured', reason };

	const dbKeys = new Set(dbXCells.map(gridCellKey));
	const unexplained = workbookX.filter(
		(cell) => !dbKeys.has(gridCellKey(cell)) && !forgivenKeys.has(gridCellKey(cell))
	);
	if (unexplained.length === 0) {
		return { kind: 'Proven', dbCount: dbXCells.length, workbookCount: workbookX.length };
	}

	return {
		kind: 'Stale',
		dbCount: dbXCells.length,
		workbookCount: workbookX.length,
		missing: unexplained.map(
			(cell) =>
				labels.get(gridCellKey(cell)) ?? [
					`learner row ${cell.rowIndex}`,
					`unmapped column ${cell.columnLetter}`
				]
		)
	};
}

/** The guard→action table. Only `Rewrite` lets the write path run. */
export function actionFor(permit: SyncPermit): SyncAction {
	switch (permit.kind) {
		case 'Proven':
			return { kind: 'Rewrite', dbCount: permit.dbCount, workbookCount: permit.workbookCount };
		case 'Stale':
			return {
				kind: 'ImportThenRecheck',
				dbCount: permit.dbCount,
				workbookCount: permit.workbookCount,
				missing: permit.missing
			};
		case 'Unmeasured':
			return { kind: 'ReadOnly', reason: permit.reason };
	}
}

/**
 * Run the guard in front of a write: evaluate, and when the workbook is ahead
 * of the database run the additive workbook→database import and evaluate again.
 *
 * The import is additive only (an X becomes an `absent` event unless the
 * database already records it; it never removes a mark), so re-evaluating
 * after it is the only thing that can turn `Stale` into `Rewrite`.
 *
 * At most two evaluations. A second `Stale` is a refusal, not a retry.
 */
export async function guardBeforeWrite(
	evaluate: () => SyncPermit | Promise<SyncPermit>,
	importMissing: (missing: [string, string][]) => void | Promise<void>
): Promise<SyncAction> {
	const first = actionFor(await evaluate());
	if (first.kind === 'Rewrite' || first.kind === 'ReadOnly') return first;
	if (first.kind === 'Aborted') return first;

	await importMissing(first.missing);

	const second = actionFor(await evaluate());
	if (second.kind === 'Rewrite' || second.kind === 'ReadOnly') return second;
	if (second.kind === 'Aborted') return second;
	return { kind: 'Aborted', message: staleAbortMessage(second.missing.length) };
}

/** The refusal shown when the workbook is still ahead after the import. */
export function staleAbortMessage(unmatchedXMarks: number): string {
	return (
		`The workbook has ${unmatchedXMarks} X marks the app has no record of. ` +
		'Nothing was changed. Restore a backup or run workbook recovery.'
	);
}

/** Name every in-scope cell, so a `Stale` permit lists learners and days. */
export function cellLabels(
	roster: readonly Sf2MonthStudentMapping[],
	dates: readonly Sf2MonthDateMappingRecord[]
): Map<string, [string, string]> {
	const labels = new Map<string, [string, string]>();
	const names = new Map<number, string>();
	for (const mapping of roster) {
		if (mapping.rowIndex > 0) names.set(mapping.rowIndex, mapping.workbookName);
	}
	for (const date of dates) {
		const sheetName = resolvedSheetName(date);
		for (const mapping of roster) {
			if (mapping.rowIndex <= 0) continue;
			const student = names.get(mapping.rowIndex);
			if (student === undefined) continue;
			labels.set(`${sheetName}!${date.columnLetter}${mapping.rowIndex}`, [student, date.date]);
		}
	}
	return labels;
}

/**
 * Measure the `X` marks a workbook holds over the scope, without modifying it.
 *
 * A failure carries its reason and no count: a failed read must resolve to
 * `Unmeasured`, never to a count of zero (zero satisfies `db >= workbook` for
 * every month, which is permission to erase the grid issued by a measurement
 * that never happened).
 */
export function measureWorkbookMarks(
	workbook: Workbook,
	roster: readonly Sf2MonthStudentMapping[],
	dates: readonly Sf2MonthDateMappingRecord[],
	scope: readonly Sf2GridCell[]
): { ok: true; xCells: Sf2GridCell[] } | { ok: false; reason: string } {
	if (dates.length === 0) return { ok: false, reason: NO_MAPPED_DATES };
	if (roster.every((mapping) => mapping.rowIndex <= 0))
		return { ok: false, reason: NO_MAPPED_LEARNERS };

	const sheets = new Map<string, Worksheet>();
	try {
		for (const date of dates) {
			const name = resolvedSheetName(date);
			if (!sheets.has(name)) sheets.set(name, getSheet(workbook, name));
		}
	} catch {
		return { ok: false, reason: WORKBOOK_NOT_READABLE };
	}

	return {
		ok: true,
		xCells: [...sheets.values()].flatMap((sheet) => workbookXCells(sheet, scope))
	};
}
