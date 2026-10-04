/**
 * Writing a month's attendance into the workbook.
 *
 * ## One open, one save
 *
 * A workbook is an in-memory value and the save is a temp-file rename, so the whole
 * write is one pass over one object. That is what lets the formula marks be computed
 * from the day grid *after* the `X` marks are in it rather than from a guess at what
 * they will be.
 *
 * ## The order is load-bearing
 *
 * 1. The literal marks: the `X`s the database proves, the blanks of every other
 *    mapped cell of the same day (present by default, so a withdrawn absence
 *    actually goes), and the blanks the differential clear allows.
 * 2. The TOTAL Per Day and AM/AO formulas, each with the value Excel caches for
 *    it, plus a corrected `AW5` day count — computed from a **read-back** of the
 *    grid the literals just wrote. Computing them from the pre-write grid is how a
 *    workbook ends up caching a count its own cells contradict.
 *
 * ## Bundled-template only
 *
 * Rust gated the formula rewrite on `source_hash.starts_with("bundled-")` so an
 * imported school's own workbook kept its original formulas. Under migration spec
 * D18 the import flow is gone: every workbook the app holds is a copy of the
 * bundled template, so the gate has nothing left to distinguish and is not
 * reproduced.
 */

import type { Worksheet } from 'exceljs';
import type { Workbook } from 'exceljs';
import { internal } from '$lib/db';
import {
	learnerAbsentPresentFormulaMarks,
	totalFormulaMarks,
	type Sf2DayColumn,
	type Sf2GridSource
} from '$lib/features/excel/formula-marks';
import { applyMarks, writeFormulaMarks, writeMarksForce } from '$lib/features/excel/marks';
import {
	SF2_FIRST_LEARNER_ROW,
	SF2_ITEM_NUMBER_COLUMN,
	SF2_METADATA_CELLS,
	SF2_NAME_COLUMN
} from '$lib/features/excel/constants';
import type { Sf2CellMark, Sf2WorkbookMetadata } from '$lib/features/excel/types';
import {
	activateSheet,
	cellAddress,
	getSheet,
	openWorkbook,
	readDayGrid,
	saveWorkbookAtomic,
	sf2MonthlySheets
} from '$lib/features/excel/workbook';
import { bestSf2MonthlySheet, setSf2MonthDates, sf2MonthNumber } from '../calendar';
import { NO_PROGRESS, emitSf2WriteStep } from '$lib/features/sf2/progress';
import type { Sf2ProgressReporter } from '$lib/features/sf2/progress';
import { attendanceMarksForDay } from '$lib/features/sf2/logic';
import { totalRowsOnSheet } from '$lib/features/sf2/month/workbook-sheets';
import { resolvedSheetName } from '$lib/features/sf2/month/templates';
import type { Sf2MonthDateMappingRecord } from '$lib/features/sf2/month/templates';
import type { Sf2MonthStudentMapping } from '$lib/features/sf2/month/students';
import {
	attendanceScopeCells,
	differentialClearMarks,
	gridCellFromMark,
	mappedAttendanceRows,
	workbookXCells,
	type Sf2GridCell
} from './attendance-marks';

/** How many marks one progress tick covers. */
export const WRITE_CHUNK_SIZE = 100;

/** Everything a write to one month's grid needs, resolved for that month. */
export type Sf2MonthWriteTarget = {
	/** The one workbook every month of the class lives in. */
	sourcePath: string;
	/** The worksheet to make active before the file is handed to the user. */
	sheetName: string;
	roster: readonly Sf2MonthStudentMapping[];
	/** This month's day columns only. */
	dates: readonly Sf2MonthDateMappingRecord[];
};

/**
 * The marks one day's mapped rows get: `X` for an explicit absence, blank for
 * everyone else.
 */
export function marksForDay(
	absent: ReadonlySet<string>,
	roster: readonly Sf2MonthStudentMapping[],
	mapping: Sf2MonthDateMappingRecord
): Sf2CellMark[] {
	const sheetName = resolvedSheetName(mapping);
	const students = roster
		.filter((entry) => entry.rowIndex > 0)
		.map((entry) => ({ studentId: entry.studentId, sheetName, rowIndex: entry.rowIndex }));
	return attendanceMarksForDay(students, absent, mapping.columnLetter);
}

/**
 * The `X` marks for every mapped day that has at least one recorded absence.
 *
 * A day with no absences is skipped rather than written as a column of blanks:
 * attendance was never taken for it, and blanking it would be the app deciding
 * something the database does not say.
 */
export function exportAttendanceMarks(params: {
	dates: readonly Sf2MonthDateMappingRecord[];
	roster: readonly Sf2MonthStudentMapping[];
	absentIdsFor: (date: string) => ReadonlySet<string>;
}): Sf2CellMark[] {
	const marks: Sf2CellMark[] = [];
	for (const mapping of params.dates) {
		const absent = params.absentIdsFor(mapping.date);
		if (absent.size === 0) continue;
		marks.push(...marksForDay(absent, params.roster, mapping));
	}
	return marks;
}

/** The day columns of one month, grouped by the worksheet they are written to. */
export function daysBySheet(
	dates: readonly Sf2MonthDateMappingRecord[]
): Map<string, Sf2DayColumn[]> {
	const grouped = new Map<string, Sf2DayColumn[]>();
	for (const date of dates) {
		const sheetName = resolvedSheetName(date);
		const column: Sf2DayColumn = { sheetName, column: date.columnLetter };
		const existing = grouped.get(sheetName);
		if (existing === undefined) grouped.set(sheetName, [column]);
		else existing.push(column);
	}
	return grouped;
}

/** The learner rows of a month, and how many of each gender block. */
export function rosterShape(roster: readonly Sf2MonthStudentMapping[]): {
	rows: { row: number }[];
	maleCount: number;
	femaleCount: number;
} {
	const rows = mappedAttendanceRows(roster.map((mapping) => mapping.rowIndex)).map((row) => ({
		row
	}));
	const femaleCount = roster.filter(
		(mapping) => mapping.rowIndex > 0 && mapping.genderBlock?.toUpperCase() === 'FEMALE'
	).length;
	return { rows, maleCount: rows.length - femaleCount, femaleCount };
}

/** The three TOTAL rows of a worksheet, as the formulas address them. */
export function totalsOn(sheet: Worksheet): {
	maleTotalRow: number;
	femaleTotalRow: number;
	combinedTotalRow: number;
} {
	const rows = totalRowsOnSheet(sheet);
	return { ...rows, combinedTotalRow: rows.femaleTotalRow + 1 };
}

/** One named phase of the write, so the progress bar can describe it. */
type WritePhase = { label: string; marks: readonly Sf2CellMark[] };

/**
 * Write a month's attendance, and everything that has to agree with it, into the
 * workbook. Returns how many `X` marks were written.
 *
 * `absentIdsFor` is the only database-shaped input: the learners with an explicit
 * absence on a given `YYYY-MM-DD`. Taking it as a callback keeps the workbook
 * arithmetic testable without a database, and makes it visible which days the
 * caller had to ask the database about.
 */
export async function writeAttendanceToWorkbook(params: {
	target: Sf2MonthWriteTarget;
	absentIdsFor: (date: string) => ReadonlySet<string>;
	progress?: Sf2ProgressReporter;
}): Promise<number> {
	const { target, absentIdsFor, progress = NO_PROGRESS } = params;
	const workbook = await openWorkbook(target.sourcePath);
	const parts = collectMonthWriteParts(workbook, target, absentIdsFor);
	applyMonthWriteParts(workbook, target, parts, progress);
	if (target.sheetName !== '') activateSheet(workbook, target.sheetName);
	await saveWorkbookAtomic(workbook, target.sourcePath);
	return parts.marks.length;
}

/** The four mark lists of one month write, in the order they are reported. */
type Sf2MonthWriteParts = {
	clearMarks: Sf2CellMark[];
	marks: Sf2CellMark[];
	formulaMarks: Sf2CellMark[];
	staticMarks: Sf2CellMark[];
};

/**
 * Compute a month's marks off an opened workbook: the literals the database
 * proves, the blanks the differential clear allows, and the formulas read back
 * off the grid they will agree with.
 *
 * `measuredX` is the guard's own measurement, reused so the write does not
 * re-read what the permit was just granted on.
 */
function collectMonthWriteParts(
	workbook: Parameters<typeof getSheet>[0],
	target: Sf2MonthWriteTarget,
	absentIdsFor: (date: string) => ReadonlySet<string>,
	measuredX?: readonly Sf2GridCell[]
): Sf2MonthWriteParts {
	const scope = attendanceScopeCells(target.roster, target.dates);
	const workbookX =
		measuredX ??
		[...daysBySheet(target.dates).keys()].flatMap((name) =>
			workbookXCells(getSheet(workbook, name), scope)
		);

	const marks = exportAttendanceMarks({
		dates: target.dates,
		roster: target.roster,
		absentIdsFor
	});
	const clearMarks = differentialClearMarks(
		scope,
		marks.map(gridCellFromMark).filter((cell) => cell !== undefined),
		workbookX
	);

	// Literals first, then the formulas read back off them.
	const formulas = monthFormulaMarks(workbook, target);
	return {
		clearMarks,
		marks,
		formulaMarks: formulas.formulaMarks,
		staticMarks: formulas.staticMarks
	};
}

/** Write computed parts into an opened workbook and report the phased progress. */
function applyMonthWriteParts(
	workbook: Parameters<typeof getSheet>[0],
	target: Sf2MonthWriteTarget,
	parts: Sf2MonthWriteParts,
	progress: Sf2ProgressReporter = NO_PROGRESS
): void {
	const phases = writePhases({
		clearMarks: parts.clearMarks,
		marks: parts.marks,
		formulaMarks: parts.formulaMarks,
		staticMarks: parts.staticMarks
	});

	writeMarksForce(workbook, [...parts.clearMarks, ...parts.marks]);
	reportWriteProgress(progress, phases);
	writeFormulaMarks(workbook, parts.formulaMarks);
	writeMarksForce(workbook, parts.staticMarks);

	if (target.sheetName !== '') activateSheet(workbook, target.sheetName);
}

/**
 * The TOTAL Per Day, AM/AO and `AW5` marks for a month, read back off the grid
 * the literals already wrote.
 *
 * The read-back is the point: `AVERAGE` over the TOTAL rows and `COUNTIF` over the
 * day block both read what is in the cells, so their cached values have to be
 * computed from the written workbook rather than from the marks we intended.
 */
export function monthFormulaMarks(
	workbook: Parameters<typeof getSheet>[0],
	target: Sf2MonthWriteTarget
): { formulaMarks: Sf2CellMark[]; staticMarks: Sf2CellMark[] } {
	const formulaMarks: Sf2CellMark[] = [];
	const staticMarks: Sf2CellMark[] = [];
	const shape = rosterShape(target.roster);

	for (const [sheetName, days] of daysBySheet(target.dates)) {
		const sheet = getSheet(workbook, sheetName);
		const totals = totalsOn(sheet);
		const gridFor: Sf2GridSource = () =>
			readDayGrid(sheet, SF2_FIRST_LEARNER_ROW, totals.combinedTotalRow);

		formulaMarks.push(
			...totalFormulaMarks(days, shape.maleCount, shape.femaleCount, totals, gridFor)
		);
		const absentPresent = learnerAbsentPresentFormulaMarks(
			[sheetName],
			shape.rows,
			shape.maleCount,
			shape.femaleCount,
			days.length,
			totals,
			gridFor
		);
		formulaMarks.push(...absentPresent.formulaMarks);
		staticMarks.push(...absentPresent.staticMarks);
	}
	return { formulaMarks, staticMarks };
}

/**
 * Report the write phase in chunks, one call per chunk.
 *
 * Rust interleaved the COM writes with the progress ticks; ExcelJS writes into
 * memory and serialises once, so the ticks are emitted once the phase's marks are
 * all in the workbook rather than during the write. The phase labels and the 61–69
 * window are unchanged, so the bar lands where it used to — and a teacher watching
 * it still sees the write happen.
 */
export function reportWriteProgress(
	progress: Sf2ProgressReporter,
	phases: readonly WritePhase[]
): void {
	const chunks = phases.map((phase) => Math.ceil(phase.marks.length / WRITE_CHUNK_SIZE));
	const totalUnits = Math.max(
		chunks.reduce((sum, count) => sum + count, 0),
		1
	);
	let unitsDone = 0;
	emitSf2WriteStep(progress, 0, totalUnits, 'Preparing the workbook…');
	for (const [index, phase] of phases.entries()) {
		for (let chunk = 1; chunk <= chunks[index]; chunk += 1) {
			unitsDone += 1;
			emitSf2WriteStep(
				progress,
				unitsDone,
				totalUnits,
				`${phase.label} (${chunk}/${chunks[index]})…`
			);
		}
	}
}

/** The phases of a write, in the order they are reported. */
export function writePhases(params: {
	clearMarks: readonly Sf2CellMark[];
	marks: readonly Sf2CellMark[];
	formulaMarks: readonly Sf2CellMark[];
	staticMarks: readonly Sf2CellMark[];
}): WritePhase[] {
	return [
		{ label: 'Clearing withdrawn marks', marks: params.clearMarks },
		{ label: 'Writing attendance marks', marks: params.marks },
		{ label: 'Updating formulas', marks: params.formulaMarks },
		{ label: 'Writing totals', marks: params.staticMarks }
	];
}

// ── The open path: layout plus marks in one open, one save ───────────────────

/** The header and calendar an open writes before the marks. */
export type Sf2OpenLayout = {
	metadata: Sf2WorkbookMetadata;
	/** The month's first attendance day, already resolved from the month row. */
	firstSchoolDay: number;
};

/**
 * The worksheet an open writes: the one named `targetName`, or — for a
 * single-sheet workbook or a school's own variant naming — the most populated
 * monthly sheet, so the header lands somewhere real rather than being
 * silently dropped.
 *
 * Made visible and renamed (31-char cap, no uniquify counter: the caller only
 * renames because no sheet carries the name), then the landing tab.
 */
function resolveOpenSheet(
	workbook: Workbook,
	targetName: string
): { sheet: Worksheet; sheetName: string } {
	const monthlySheets = sf2MonthlySheets(workbook);
	const sheet =
		monthlySheets.find((candidate) => candidate.name === targetName) ??
		bestSf2MonthlySheet(monthlySheets) ??
		monthlySheets[0];
	if (sheet === undefined) throw internal('SF2 workbook has no monthly worksheet to configure');

	sheet.state = 'visible';
	sheet.name = targetName.slice(0, 31);
	return { sheet, sheetName: sheet.name };
}

/**
 * The eight header fields on one sheet, through the merge master only.
 *
 * Target sheet only: every month row carries its own header, so writing one
 * month's metadata across all twelve sheets would stamp the wrong school year
 * on eleven of them.
 */
function metadataMarksForSheet(sheetName: string, metadata: Sf2WorkbookMetadata): Sf2CellMark[] {
	const entries: [keyof typeof SF2_METADATA_CELLS, string][] = [
		['schoolId', metadata.schoolId],
		['schoolYear', metadata.schoolYear],
		['reportMonth', metadata.reportMonth],
		['schoolName', metadata.schoolName],
		['gradeLevel', metadata.gradeLevel],
		['section', metadata.section],
		['adviserSignature', metadata.adviserName],
		['adviserPrintedName', metadata.adviserName],
		['schoolHeadPrintedName', metadata.schoolHeadName]
	];
	return entries.map(([field, value]) => {
		const { row, column } = SF2_METADATA_CELLS[field];
		return { sheetName, address: cellAddress(row, column), value };
	});
}

/**
 * The roster rows an open writes: item numbers and names for the mapped
 * learner rows, through the merge master only.
 *
 * Mapped rows only — no row is invented for an unmapped learner, and no vacated
 * row is cleared here (that is the roster sync's job). Item numbers count from
 * 1 within each gender block, the way the form numbers them.
 */
function rosterMarksForSheet(
	sheetName: string,
	roster: readonly Sf2MonthStudentMapping[]
): Sf2CellMark[] {
	const mapped = roster
		.filter((entry) => entry.rowIndex > 0)
		.sort((left, right) => left.rowIndex - right.rowIndex);
	const itemByRow = new Map<number, number>();
	for (const block of ['MALE', 'FEMALE']) {
		let item = 0;
		for (const entry of mapped) {
			if (entry.genderBlock?.toUpperCase() !== block) continue;
			item += 1;
			itemByRow.set(entry.rowIndex, item);
		}
	}

	return mapped.flatMap((entry) => {
		const marks: Sf2CellMark[] = [
			{
				sheetName,
				address: cellAddress(entry.rowIndex, SF2_NAME_COLUMN),
				value: entry.workbookName.trim()
			}
		];
		const item = itemByRow.get(entry.rowIndex);
		if (item !== undefined) {
			marks.push({
				sheetName,
				address: cellAddress(entry.rowIndex, SF2_ITEM_NUMBER_COLUMN),
				value: String(item)
			});
		}
		return marks;
	});
}

/**
 * Write an open month into an opened workbook: sheet, header, dates, roster,
 * marks, formulas — then the landing tab. The caller saves.
 *
 * Returns what was written and where, so the caller can stamp and report.
 */
export function writeOpenMonthIntoWorkbook(params: {
	workbook: Workbook;
	target: Sf2MonthWriteTarget;
	/** Canonical month name and calendar year, for the date header. */
	month: { reportMonth: string; reportYear: number };
	layout: Sf2OpenLayout;
	absentIdsFor: (date: string) => ReadonlySet<string>;
	measuredX?: readonly Sf2GridCell[];
	progress?: Sf2ProgressReporter;
}): { marksWritten: number; sheetName: string; droppedSchoolDays: number[] } {
	const {
		workbook,
		target,
		month,
		layout,
		absentIdsFor,
		measuredX,
		progress = NO_PROGRESS
	} = params;

	const { sheet, sheetName } = resolveOpenSheet(workbook, target.sheetName);
	const onSheet: Sf2MonthWriteTarget = {
		...target,
		sheetName,
		dates: target.dates.map((date) => ({ ...date, sheetName }))
	};

	applyMarks(workbook, metadataMarksForSheet(sheetName, layout.metadata), { textFormat: true });

	const reportMonth = sf2MonthNumber(month.reportMonth);
	if (reportMonth === undefined)
		throw internal(`Report month is not a month: ${month.reportMonth}`);
	// Strict: an out-of-range or weekend first day refuses rather than falling
	// back to the first Monday-Friday, which would date the grid wrong.
	const droppedSchoolDays = setSf2MonthDates(
		sheet,
		month.reportYear,
		reportMonth,
		layout.firstSchoolDay
	);

	writeMarksForce(workbook, rosterMarksForSheet(sheetName, target.roster));

	const parts = collectMonthWriteParts(workbook, onSheet, absentIdsFor, measuredX);
	applyMonthWriteParts(workbook, onSheet, parts, progress);
	return { marksWritten: parts.marks.length, sheetName, droppedSchoolDays };
}
