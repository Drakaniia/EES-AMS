/**
 * The SF2 attendance service.
 *
 * Five modules, one concern each:
 *
 * - `attendance-events` — writing and reading a learner's mark for one day.
 * - `attendance-marks` — which cells of the grid a sync may write, and the
 *   differential clear that bounds it.
 * - `attendance-write` — the workbook half: the marks, the formulas, the save.
 * - `attendance-import` — reading `X` marks back out of the workbook.
 * - `attendance-service` — the four commands `$lib/api/sf2.ts` calls, plus
 *   `write-context`, which resolves the month a write is for.
 */

export {
	SF2_PREVIEW_CORRECTION,
	absentEventsForDay,
	absentStudentIds,
	attendanceTimestampForDate,
	hasAbsentEventForDay,
	hasEventOfTypeForDay,
	hasPresentEventForDay,
	localDayBoundsTimestampsForDate,
	parseClock,
	presentEventsForDay,
	presentStudentIds,
	setAttendanceEventForDay
} from './attendance-events';

export {
	attendanceScopeCells,
	differentialClearMarks,
	gridCellAddress,
	gridCellFromMark,
	gridCellKey,
	mappedAttendanceRows,
	workbookXCells
} from './attendance-marks';
export type { Sf2GridCell } from './attendance-marks';

export {
	daysBySheet,
	exportAttendanceMarks,
	marksForDay,
	monthFormulaMarks,
	reportWriteProgress,
	rosterShape,
	totalsOn,
	writeAttendanceToWorkbook,
	writePhases,
	WRITE_CHUNK_SIZE
} from './attendance-write';
export type { Sf2MonthWriteTarget } from './attendance-write';

export {
	emptyImportOutcome,
	gridCellsToScan,
	importAbsentMarks,
	isAbsentMark,
	IMPORT_REASON
} from './attendance-import';
export type { Sf2ScanCell } from './attendance-import';

export {
	absentIdsByDate,
	monthAbsences,
	resolveMonthWriteContext,
	resolveSchoolYear,
	UNKNOWN_MONTH_MESSAGE
} from './write-context';
export type { Sf2MonthWriteContext } from './write-context';

export {
	importAbsentMarksFromWorkbook,
	NO_MONTH_SELECTED_MESSAGE,
	presentAllPreviewAttendance,
	setPreviewAttendanceLightweight,
	syncAndOpenSf2Workbook,
	WORKBOOK_MISSING_MESSAGE
} from './attendance-service';

export type { Sf2AttendanceImportOutcome } from '$lib/types';
