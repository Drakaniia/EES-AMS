/**
 * The read-only SF2 mark diagnostic — spec 0 A5, brief R1.
 *
 * For each of the school's twelve months: how many `X` marks the workbook holds, how
 * many absences the database holds, and which specific `(learner, day)` cells each side
 * has and the other does not. Plus a verdict, and the row counts of the legacy and
 * per-month mapping tables.
 *
 * Entry point: {@link diagnoseSf2Marks}. Everything else here is either the shape of
 * the answer (`./model`), the comparison itself (`./compare`), the read-only pass over a
 * workbook (`./probe`), the database reads and their statements (`./db-read`, `./sql`),
 * or the lists that keep a verdict honest (`./report`).
 */

export { diagnoseSf2Marks } from './diagnose';

export {
	incomparableSummary,
	isComparable,
	permitsWrite,
	unmeasuredCounts,
	unmeasuredMonth,
	verdictFor,
	verdictReason,
	type AbsentRecord,
	type EventTypeCount,
	type MappingSource,
	type MappingTableState,
	type MarkCell,
	type MarkCounts,
	type MarkSourceStatus,
	type MarkVerdict,
	type MonthMarkComparison,
	type RosterResolution,
	type RosterRow,
	type Sf2MarkDiagnostic,
	type SheetDayGridSummary,
	type TableRowCount,
	type UnplacedSheet,
	type WorkbookFileReport
} from './model';

export {
	absentCountInMonth,
	attendanceScopeCells,
	buildDateMappings,
	buildStudentMappings,
	cellAddress,
	databaseXCells,
	diffCells,
	label,
	monthFromMeasurement,
	rawMarkAddress,
	workbookXCellsInScope,
	type DateMapping,
	type GridCell,
	type RawMark,
	type StudentMapping
} from './compare';

export {
	markCount,
	nameAt,
	probeWorkbook,
	sheetForMonth,
	yearFromSheetName,
	type RawSheet,
	type RawWorkbook
} from './probe';

export {
	activeClassId,
	anchorTemplate,
	readSnapshot,
	schoolYear,
	type DbSnapshot,
	type LegacyTemplateRow,
	type MonthTemplateRow,
	type StudentRow
} from './db-read';

export { classAbsences, matchRosterByName, rosterFor, rosterForSheet } from './roster';

export {
	isFile,
	probeAll,
	referencedWorkbookPath,
	resolveWorkbookDir,
	workbookCandidates
} from './candidates';

export {
	ABSENT_EVENTS_SQL,
	ABSENT_EVENT_TYPE,
	DIAGNOSTIC_STATEMENTS,
	LEGACY_TABLES,
	PER_MONTH_TABLES
} from './sql';

export {
	consumedKey,
	excelFailures,
	unplacedSheets,
	workbookReports,
	type WorkbookProbe
} from './report';
