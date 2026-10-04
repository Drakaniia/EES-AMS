/**
 * The shape of the mark diagnostic, and the rules that turn measurements into a
 * verdict — the port of `src-tauri/src/sf2/diagnose/model.rs`.
 *
 * Everything here is data. The one behavioural rule - what a month's status is
 * allowed to claim - lives in {@link unmeasuredMonth}, because that is where the
 * difference between "the workbook holds no marks" and "the workbook could not be
 * read" is decided, and getting it wrong is the one failure this whole module
 * exists to prevent: **a false zero.**
 *
 * ## Why `undefined` is everywhere
 *
 * {@link MarkCounts} and {@link MonthMarkComparison} hold `undefined` on every
 * measured number rather than defaulting to `0`. A count of zero is a claim - "I
 * looked and there is nothing" - and this module is only allowed to make that claim
 * when it actually looked. Every other state is a refusal:
 *
 * | status              | meaning                                                        |
 * | ------------------- | -------------------------------------------------------------- |
 * | `NoSheet`           | the workbook has no worksheet for this month                    |
 * | `NoMappings`        | the worksheet exists but nothing on it resolves to a day/learner |
 * | `ExcelUnavailable`  | the file could not be opened, or the user has it open           |
 * | `WorkbookMissing`   | the file the database points at is not on disk                  |
 *
 * Any of those is {@link MarkSourceStatus} other than `Comparable`, and an
 * incomparable month makes the whole verdict incomparable. The workbook is the
 * only known copy of this user's marks (spec 0 A5); answering "the database is
 * complete" for a month nobody managed to read is how that copy gets cleared.
 *
 * ## Determinism
 *
 * Rust's `HashSet` iteration order was arbitrary, so every list this module used to
 * produce came out in a different order per run. Nothing here uses a hash set: the
 * comparators below are explicit and the sort orders are stated at each call site,
 * so the same inputs always render the same UI list.
 */

/** One absence the database holds: which child, on which local day. */
export interface AbsentRecord {
	studentId: string;
	classId?: string;
	/** `YYYY-MM-DD`, already local. */
	date: string;
}

/** One learner row of a workbook, as the database believes it to be. */
export interface RosterRow {
	studentId: string;
	workbookName: string;
	rowIndex: number;
}

/** Where the roster and grid the comparison used came from. */
export type MappingSource = 'perMonthTables' | 'legacyTables' | 'none';

/** How a month's roster rows were decided. */
export type RosterResolution = 'databaseRowMappings' | 'workbookNameMatch' | 'unresolved';

/** One cell of the attendance block, in the vocabulary the writer shares. */
export interface MarkCell {
	/** The child's name, as the workbook spells it. */
	studentName: string;
	/** `YYYY-MM-DD`, or a sentence saying why the cell has no date. */
	date: string;
	/** The worksheet the cell is on. */
	sheetName: string;
	/** The A1 address, e.g. `AL47`. */
	cellAddress: string;
}

/** Could this month be measured at all? */
export type MarkSourceStatus =
	| 'Comparable'
	| 'WorkbookMissing'
	| 'ExcelUnavailable'
	| 'NoSheet'
	| 'NoMappings';

/** Is this month's measurement usable? */
export function isComparable(status: MarkSourceStatus): boolean {
	return status === 'Comparable';
}

/**
 * The measured numbers for one month.
 *
 * Every count is optional, and absent means *not measured*, never zero.
 */
export interface MarkCounts {
	/**
	 * `X` cells the workbook holds, in scope: the mapped learner rows and the mapped
	 * day columns of this month's sheet.
	 */
	workbookXCount?: number;
	/**
	 * `absent` events the database holds whose local date falls in this month, for
	 * the class this workbook is for. Not scoped to the grid, so it can exceed
	 * {@link MarkCounts.dbMappedAbsentCount}.
	 */
	dbAbsentCount?: number;
	/**
	 * The subset of {@link MarkCounts.dbAbsentCount} the grid can actually hold: an
	 * absence on a day the month has no column for, or for a learner with no roster
	 * row, is in the first count and not in this one.
	 */
	dbMappedAbsentCount?: number;
	/** Learner rows x day columns actually inspected. Absent when nothing was. */
	cellsScanned?: number;
}

/** What the diagnostic found for a month it could not read: nothing, not zeroes. */
export function unmeasuredCounts(): MarkCounts {
	return {};
}

/** One month of the comparison. */
export interface MonthMarkComparison {
	/** Canonical uppercase month name, e.g. `SEPTEMBER`. */
	reportMonth: string;
	reportYear: number;
	counts: MarkCounts;
	cellsOnlyInWorkbook: MarkCell[];
	cellsOnlyInDatabase: MarkCell[];
	sourceStatus: MarkSourceStatus;
	/** Why the status is what it is, in a sentence a teacher can read. */
	reason: string;
	/** The worksheet this month was measured on, when there was one. */
	sheetName?: string;
	/**
	 * The file that worksheet is on. Two months measured on the same file is the
	 * normal case; two different files is worth seeing.
	 */
	workbookPath?: string;
	mappingSource: MappingSource;
	/** How the roster rows used for this month were decided. */
	rosterResolution: RosterResolution;
	rosterRows: number;
	dayColumns: number;
}

/**
 * A month that could not be measured, and why.
 *
 * The only constructor that produces a status without a count. A caller cannot
 * reach `sourceStatus: 'Comparable'` through here, so the two can never be got out
 * of step.
 */
export function unmeasuredMonth(
	reportMonth: string,
	reportYear: number,
	status: MarkSourceStatus,
	reason: string
): MonthMarkComparison {
	return {
		reportMonth,
		reportYear,
		counts: unmeasuredCounts(),
		cellsOnlyInWorkbook: [],
		cellsOnlyInDatabase: [],
		sourceStatus: status,
		reason,
		mappingSource: 'none',
		rosterResolution: 'unresolved',
		rosterRows: 0,
		dayColumns: 0
	};
}

/** What the twelve months add up to. */
export type MarkVerdict =
	/** The workbook holds `X` marks the database has no record of. */
	| 'WorkbookIsSourceOfTruth'
	/** Every month was measurable and the database held at least every mark. */
	| 'DatabaseIsSourceOfTruth'
	/** At least one month could not be measured. No conclusion is drawn. */
	| 'Incomparable';

/**
 * May a write path act on this verdict?
 *
 * `true` only for `DatabaseIsSourceOfTruth`, and even that only clears the
 * *comparison*. Under D15 there is no destructive-sync guard left to clear: a write
 * goes to a temp file and is renamed over the target, so this is advisory for the
 * UI rather than a gate a writer consults.
 */
export function permitsWrite(verdict: MarkVerdict): boolean {
	return verdict === 'DatabaseIsSourceOfTruth';
}

/** One worksheet the diagnostic found but could not place in a month. */
export interface UnplacedSheet {
	workbookPath: string;
	sheetName: string;
	visible: boolean;
	/** `X` cells on the sheet, over the whole day-column block. */
	unrowedXCount: number;
	/** Why the sheet could not be compared against a month. */
	reason: string;
	/** Every `X` on the sheet, named by the sheet's own roster, with its address. */
	marks: MarkCell[];
}

/** One workbook in the workbook directory, and what was found on it. */
export interface WorkbookFileReport {
	path: string;
	/** Is this the file the database's template row points at? */
	isReferencedByDatabase: boolean;
	sheetCount: number;
	monthSheetsMeasured: number;
	/** Total `X` cells over every sheet in the file, day-column block only. */
	totalXCount: number;
	/** Set when the file could not be opened at all. */
	readError?: string;
}

/**
 * Row counts of one table, or `undefined` when the table does not exist.
 *
 * Absent is not `0`: a database at schema v18 has no per-month tables, and
 * reporting those as empty would say "the backfill copied nothing" when the truth is
 * "the backfill cannot have run yet".
 */
export interface TableRowCount {
	table: string;
	exists: boolean;
	rows?: number;
}

/** The legacy and per-month mapping tables, as they actually stand. */
export interface MappingTableState {
	legacy: TableRowCount[];
	perMonth: TableRowCount[];
	/** The legacy `sf2_date_mappings` grouped by the sheet it names. */
	legacyDateMappingSheets: SheetDayGridSummary[];
	/** The per-month `sf2_month_date_mappings` grouped by the month it covers. */
	monthDateMappingGrids: SheetDayGridSummary[];
}

/** One stored grid, summarised by what it claims to cover. */
export interface SheetDayGridSummary {
	/**
	 * The worksheet, for the legacy table. Empty for the per-month table, which has
	 * no `sheet_name` column (spec 0 A4).
	 */
	sheetName: string;
	/** `YYYY-MM`, derived from the mapping rows' own dates. */
	yearMonth: string;
	firstDate: string;
	lastDate: string;
	dayColumns: number;
}

/** `events` rows for one event type. */
export interface EventTypeCount {
	eventType: string;
	rows: number;
}

/** The whole answer. */
export interface Sf2MarkDiagnostic {
	/** Unix seconds, when the diagnostic ran. */
	generatedAt: number;
	databasePath: string;
	/** `PRAGMA user_version` of that database. */
	schemaVersion?: number;
	/** The workbook file the database's template row points at. */
	workbookPath?: string;
	/** The directory every workbook was looked for in. */
	workbookDir: string;
	activeClassId?: string;
	schoolYear?: string;
	/** The month the database's own template row names. */
	storedReportMonth?: string;
	/** Where the roster and grid the comparison used came from. */
	mappingSource: MappingSource;
	months: MonthMarkComparison[];
	verdict: MarkVerdict;
	/** One sentence, in the vocabulary above, saying what the verdict rests on. */
	verdictReason: string;
	/**
	 * `"{MONTH} {year}: {status} - {reason}"` for every month that could not be
	 * measured. Never empty when the verdict is `Incomparable` and there was at least
	 * one month to try.
	 */
	incomparableMonths: string[];
	/**
	 * Worksheets that hold `X` cells and cannot be tied to a month. This is the list
	 * that keeps a "the database is complete" answer honest.
	 */
	unplacedSheets: UnplacedSheet[];
	workbooks: WorkbookFileReport[];
	tables: MappingTableState;
	/** `events` rows per event type, so a total can be checked against the figures. */
	eventCounts: EventTypeCount[];
	/** Total `absent` events in the database. */
	totalAbsentEvents: number;
	/**
	 * Absences that belong to no class this diagnostic knows about - either recorded
	 * against a different `class_id`, or against none. Nothing will ever place these
	 * on a grid, so they are counted separately rather than inflating a month's
	 * number.
	 */
	absentEventsWithoutClass: number;
}

/**
 * What the twelve months say.
 *
 * Three rules, in this order, and the order is the point:
 *
 * 1. **Any month the workbook holds marks the database lacks wins.** Not because it
 *    is the most likely answer - because it is the one that must never be lost to a
 *    nicer-sounding verdict.
 * 2. `DatabaseIsSourceOfTruth` requires **all** months to be comparable. A school
 *    year with eleven months that could not be read is not a school year the database
 *    has been shown to cover.
 * 3. Anything else is `Incomparable`, and the caller is told which months and why.
 */
export function verdictFor(months: readonly MonthMarkComparison[]): MarkVerdict {
	if (months.some((month) => month.cellsOnlyInWorkbook.length > 0)) {
		return 'WorkbookIsSourceOfTruth';
	}
	if (months.length === 0) {
		return 'Incomparable';
	}
	if (months.every((month) => isComparable(month.sourceStatus))) {
		return 'DatabaseIsSourceOfTruth';
	}
	return 'Incomparable';
}

/** The sentence that goes with a {@link MarkVerdict}. */
export function verdictReason(
	verdict: MarkVerdict,
	months: readonly MonthMarkComparison[],
	unplacedXCells: number
): string {
	if (verdict === 'WorkbookIsSourceOfTruth') {
		const missing = months.reduce((sum, month) => sum + month.cellsOnlyInWorkbook.length, 0);
		return (
			`${missing} X mark(s) in the workbook have no record in the database. The workbook is ` +
			'the only copy of those marks: import them before anything is written.'
		);
	}
	if (verdict === 'DatabaseIsSourceOfTruth') {
		return (
			`All ${months.length} months were measurable and the database holds every mark the ` +
			'workbook shows, on the same cells.'
		);
	}
	const unmeasured = months
		.filter((month) => !isComparable(month.sourceStatus))
		.map((month) => `${month.reportMonth} ${month.reportYear} (${month.sourceStatus})`);
	const reason =
		unmeasured.length === 0
			? 'No month could be measured.'
			: `${unmeasured.length} of ${months.length} month(s) could not be measured: ` +
				`${unmeasured.join(', ')}. No conclusion is drawn.`;
	if (unplacedXCells > 0) {
		return (
			`${reason} A further ${unplacedXCells} X cell(s) sit on worksheet(s) that cannot be ` +
			'tied to any month, so they were not compared either.'
		);
	}
	return reason;
}

/**
 * `"{MONTH} {year}: {status} - {reason}"`, for the months a reader must be told
 * about.
 */
export function incomparableSummary(month: MonthMarkComparison): string {
	return `${month.reportMonth} ${month.reportYear}: ${month.sourceStatus} - ${month.reason}`;
}
