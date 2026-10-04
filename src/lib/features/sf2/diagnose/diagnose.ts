/**
 * Measure, per month, what the SF2 workbook holds against what the database holds —
 * the port of `src-tauri/src/sf2/diagnose/mod.rs`.
 *
 * ## What it answers
 *
 * For each of the school's twelve months: how many `X` marks the workbook holds, how
 * many absences the database holds, and which specific `(learner, day)` cells each side
 * has and the other does not. Plus a verdict, and the row counts of the legacy and
 * per-month mapping tables.
 *
 * ## What it never does
 *
 * **It never writes.** Not to the workbook, not to the database. Every database statement
 * is in `./sql` and every one of them is a `SELECT` or a read-only `PRAGMA`; the workbook
 * is parsed into an in-memory model and nothing here holds a writer.
 *
 * ## Why the workbook is read for itself rather than trusted
 *
 * The stored day grid is exactly what a destructive sync destroys, so a diagnostic that
 * read the grid out of the database would report the damage as though it were the truth.
 * Every worksheet's day-number row is read out of the file instead, and a month whose
 * stored grid names a worksheet the file no longer has is reported as such.
 *
 * ## The false-zero rule
 *
 * A count of `0` is a claim. This module makes it only after a measurement, and every
 * other outcome — no sheet, no grid, no roster, unreadable file, no file at all — is
 * reported as a refusal with a reason. See `./model`.
 */

import { DB_FILENAME } from '$lib/db';
import { sf2MonthName, sf2ReportYear } from '$lib/features/sf2/calendar';
import {
	absentCountInMonth,
	attendanceScopeCells,
	buildDateMappings,
	buildStudentMappings,
	databaseXCells,
	monthFromMeasurement,
	workbookXCellsInScope,
	type GridCell
} from './compare';
import {
	isFile,
	probeAll,
	referencedWorkbookPath,
	resolveWorkbookDir,
	workbookCandidates
} from './candidates';
import {
	activeClassId,
	anchorTemplate,
	readSnapshot,
	schoolYear,
	type DbSnapshot
} from './db-read';
import {
	incomparableSummary,
	isComparable,
	unmeasuredMonth,
	verdictFor,
	verdictReason,
	type AbsentRecord,
	type MarkSourceStatus,
	type MarkVerdict,
	type MonthMarkComparison,
	type Sf2MarkDiagnostic
} from './model';
import { sheetForMonth, type RawSheet } from './probe';
import {
	consumedKey,
	excelFailures,
	probeSheets,
	unplacedSheets,
	unreadableCount,
	workbookReports,
	type WorkbookProbe
} from './report';
import { classAbsences, rosterFor, rosterForSheet } from './roster';

/** The twelve months of a school year. */
const SCHOOL_YEAR_MONTHS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

/**
 * Measure, per month, what the workbook holds against what the database holds.
 *
 * Nothing outside the driver's reads and the workbook parse is touched.
 */
export async function diagnoseSf2Marks(): Promise<Sf2MarkDiagnostic> {
	const snapshot = await readSnapshot();
	const workbookDir = await resolveWorkbookDir(snapshot);
	const referenced = referencedWorkbookPath(snapshot);
	const referenceMissing = referenced === undefined || !(await isFile(referenced));
	const probes = await probeAll(await workbookCandidates(referenced, workbookDir));
	// A referenced file that is not on disk is not the file any sheet was measured on, so
	// the sheets found in the directory fall back to the name-match roster.
	const measuredReference = referenceMissing ? undefined : referenced;

	const classId = activeClassId(snapshot);
	const classAbsent = classId === undefined ? [] : classAbsences(snapshot, classId);
	const year = schoolYear(snapshot) ?? '';
	const templateId = anchorTemplateId(snapshot);

	const { months, consumed } = measureMonths(
		snapshot,
		probes,
		measuredReference,
		year,
		templateId,
		classAbsent
	);

	const unplaced = unplacedSheets(probes, consumed);
	const unplacedXCells = unplaced.reduce((sum, sheet) => sum + sheet.marks.length, 0);
	const [verdict, reason] = verdictForMonths(months, unplacedXCells);
	const anchor = anchorTemplate(snapshot);

	return {
		generatedAt: Math.floor(Date.now() / 1000),
		databasePath: DB_FILENAME,
		schemaVersion: snapshot.schemaVersion,
		workbookPath: referenced,
		workbookDir,
		activeClassId: classId,
		schoolYear: schoolYear(snapshot),
		storedReportMonth: anchor?.reportMonth,
		mappingSource: anchor === undefined ? 'none' : rosterFor(snapshot, anchor.id).source,
		incomparableMonths: months
			.filter((month) => !isComparable(month.sourceStatus))
			.map(incomparableSummary),
		months,
		verdict,
		verdictReason: reason,
		unplacedSheets: unplaced,
		workbooks: workbookReports(probes, measuredReference),
		tables: snapshot.tables,
		eventCounts: snapshot.eventCounts,
		totalAbsentEvents: snapshot.totalAbsentEvents,
		absentEventsWithoutClass: Math.max(0, snapshot.totalAbsentEvents - classAbsent.length)
	};
}

/** Walk the twelve months, and report which worksheets each one consumed. */
function measureMonths(
	snapshot: DbSnapshot,
	probes: readonly WorkbookProbe[],
	referenced: string | undefined,
	year: string,
	templateId: string,
	classAbsent: readonly AbsentRecord[]
): { months: MonthMarkComparison[]; consumed: Set<string> } {
	const months: MonthMarkComparison[] = [];
	const consumed = new Set<string>();

	for (const month of SCHOOL_YEAR_MONTHS) {
		const name = sf2MonthName(month);
		const located = locateSheet(probes, month);
		if (located === undefined) {
			months.push(noSheetStatus(probes, referenced, year, month, name));
			continue;
		}
		consumed.add(consumedKey(located.path, located.sheet.sheetName));
		months.push(
			measureOneMonth(
				snapshot,
				referenced,
				year,
				templateId,
				month,
				name,
				located.path,
				located.sheet,
				classAbsent
			)
		);
	}
	return { months, consumed };
}

/** The first worksheet that names `month`, across every workbook, referenced file first. */
function locateSheet(
	probes: readonly WorkbookProbe[],
	month: number
): { path: string; sheet: RawSheet } | undefined {
	for (const probe of probes) {
		const sheet = probe.workbook === undefined ? undefined : sheetForMonth(probe.workbook, month);
		if (sheet !== undefined) return { path: probe.path, sheet };
	}
	return undefined;
}

/** The month's status when no worksheet anywhere names it. */
function noSheetStatus(
	probes: readonly WorkbookProbe[],
	referenced: string | undefined,
	year: string,
	month: number,
	name: string
): MonthMarkComparison {
	const reportYear = sf2ReportYear(year, month);

	if (referenced === undefined && probes.length === 0) {
		return unmeasuredMonth(
			name,
			reportYear,
			'WorkbookMissing',
			'The database names no SF2 workbook on disk (no template row at all), so no month ' +
				'could be measured.'
		);
	}
	const failures = excelFailures(probes);
	if (failures !== undefined) {
		return unmeasuredMonth(
			name,
			reportYear,
			'ExcelUnavailable',
			`${unreadableCount(probes)} file(s) in the workbook directory could not be read, so a ` +
				`worksheet for ${name} ${reportYear} may exist behind one of them: ${failures}. This ` +
				"is not a statement about the month's marks."
		);
	}
	const examined = probes.reduce((sum, probe) => sum + probeSheets(probe).length, 0);
	return unmeasuredMonth(
		name,
		reportYear,
		'NoSheet',
		`No worksheet in any of the ${probes.length} workbook(s) examined (${examined} sheet(s) read ` +
			`across all of them) names ${name} ${reportYear}. The workbook holds no sheet for this ` +
			'month, so nothing could be compared - which is not the same as it holding no marks.'
	);
}

/** Measure one month against the database. */
function measureOneMonth(
	snapshot: DbSnapshot,
	referenced: string | undefined,
	year: string,
	templateId: string,
	month: number,
	name: string,
	path: string,
	sheet: RawSheet,
	classAbsent: readonly AbsentRecord[]
): MonthMarkComparison {
	const isReferenced = referenced === path;
	const reportYear = sheet.yearFromName ?? sf2ReportYear(year, month);

	if (sheet.dayNumbers.length === 0) {
		return unmeasured(
			name,
			reportYear,
			'NoMappings',
			`Worksheet '${sheet.sheetName}' holds ${sheet.marks.length} X cell(s) but its day-number ` +
				'row is empty, so no cell on it can be resolved to a date. The marks are there; ' +
				'nothing can place them.',
			sheet,
			path
		);
	}

	const { roster, mappingSource, rosterResolution } = rosterForSheet(
		sheet,
		snapshot,
		templateId,
		isReferenced
	);
	if (roster.length === 0) {
		return unmeasured(
			name,
			reportYear,
			'NoMappings',
			`Worksheet '${sheet.sheetName}' has a day grid of ${sheet.dayNumbers.length} column(s) ` +
				'but no learner could be placed on a row, so no cell on it can be compared.',
			sheet,
			path
		);
	}

	const dateMappings = buildDateMappings(
		templateId,
		sheet.sheetName,
		reportYear,
		month,
		sheet.dayNumbers
	);
	const studentMappings = buildStudentMappings(templateId, roster);

	// The workbook side of the comparison, from the block the probe has already read in
	// full, over exactly the scope the writer uses.
	const scope: GridCell[] = attendanceScopeCells(studentMappings, dateMappings);
	const workbookCells = workbookXCellsInScope(sheet.marks, sheet.sheetName, scope);
	const databaseCells = databaseXCells(roster, dateMappings, classAbsent);

	return monthFromMeasurement(
		name,
		reportYear,
		sheet.sheetName,
		path,
		mappingSource,
		rosterResolution,
		roster,
		dateMappings,
		workbookCells,
		databaseCells,
		scope.length,
		absentCountInMonth(classAbsent, reportYear, month)
	);
}

/** An unmeasured month that still carries the worksheet it was refused on. */
function unmeasured(
	name: string,
	year: number,
	status: MarkSourceStatus,
	reason: string,
	sheet: RawSheet,
	path: string
): MonthMarkComparison {
	return {
		...unmeasuredMonth(name, year, status, reason),
		sheetName: sheet.sheetName,
		workbookPath: path,
		dayColumns: sheet.dayNumbers.length,
		rosterRows: sheet.rosterNames.length
	};
}

/**
 * The id the month's mappings are looked up under.
 *
 * The legacy row's id when there is one - it is the row whose `source_path` names the
 * workbook, and the per-month backfill copies that same id - and the first month row's
 * otherwise. Only the id matters here: it keys the roster, and the grid is read out of
 * the file.
 */
function anchorTemplateId(snapshot: DbSnapshot): string {
	return anchorTemplate(snapshot)?.id ?? snapshot.monthTemplates[0]?.id ?? '';
}

/**
 * `verdictReason` needs the verdict, and the verdict needs the months; this is the one
 * place both are derived so the two cannot be taken from different versions of the month
 * list.
 */
function verdictForMonths(
	months: readonly MonthMarkComparison[],
	unplacedXCells: number
): [MarkVerdict, string] {
	const verdict = verdictFor(months);
	return [verdict, verdictReason(verdict, months, unplacedXCells)];
}
