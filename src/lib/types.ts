/// Type definitions matching the Rust backend

export type StudentGender = 'male' | 'female';

export interface Student {
	id: string;
	name: string;
	gender?: StudentGender;
	classId?: string;
	createdAt: string;
}

export interface Session {
	name: string;
	startTime: string;
	endTime: string;
	lateAfter: string;
}

export interface Class {
	id: string;
	name: string;
	room?: string;
	dayStart: string;
	dayEnd: string;
	lateAfter: string;
	sessions: Session[];
	days: number[];
	createdAt: string;
}

export type AttendanceType = 'in' | 'absent';
export type AttendanceMode = 'manual';

export interface AttendanceEvent {
	id: string;
	studentId: string;
	classId?: string;
	type: AttendanceType;
	timestamp: string;
	note?: string;
	sessionKey?: string;
	overrideReason?: string;
	updatedAt?: string;
}

export interface AttendanceAuditEntry {
	id: string;
	eventId?: string;
	studentId: string;
	classId?: string;
	sessionKey?: string;
	action: 'create_override' | 'update' | 'delete';
	reason: string;
	beforeJson?: string;
	afterJson?: string;
	createdAt: string;
	actor: string;
}

export interface AuditEvent {
	id: string;
	entityType: string;
	entityId?: string;
	action: string;
	summary: string;
	beforeJson?: string;
	afterJson?: string;
	metadataJson?: string;
	createdAt: string;
	actor: string;
}

export interface Settings {
	id: string;
	dayStart: string;
	dayEnd: string;
	lateAfter: string;
	quarter: string;
	attendanceMode: AttendanceMode;
	q1Start?: string;
	q1End?: string;
	q2Start?: string;
	q2End?: string;
	q3Start?: string;
	q3End?: string;
}

export interface CreateStudentRequest {
	name: string;
	gender?: StudentGender;
	classId?: string;
}

export interface UpdateStudentRequest {
	name?: string;
	gender?: StudentGender;
	classId?: string;
}

export interface CreateClassRequest {
	name: string;
	room?: string;
	dayStart: string;
	dayEnd: string;
	lateAfter: string;
	sessions: Session[];
	days: number[];
}

export interface UpdateClassRequest {
	name?: string;
	room?: string;
	dayStart?: string;
	dayEnd?: string;
	lateAfter?: string;
	sessions?: Session[];
	days?: number[];
}

export interface CreateEventRequest {
	studentId: string;
	classId?: string;
	type: AttendanceType;
	note?: string;
	sessionKey?: string;
	overrideReason?: string;
	timestamp?: string;
}

export interface UpdateEventRequest {
	classId?: string;
	timestamp?: string;
	note?: string;
	sessionKey?: string;
	reason: string;
}

export interface ExportData {
	students: Student[];
	classes: Class[];
	events: AttendanceEvent[];
	settings: Settings[];
	auditEvents?: AuditEvent[];
	exportedAt: number;
}

export type BackupKind =
	| 'auto'
	| 'manual'
	| 'pre_restore'
	| 'pre_wipe'
	| 'pre_install'
	| 'manual_workbooks'
	| 'unknown';

/**
 * One workbook listed in a backup's `manifest.json`. `xCount` is the number of
 * `"X"` cells the workbook held when the backup was taken — the proof the restore
 * dialog compares against the database's own absence count.
 */
export interface BackupWorkbookPreview {
	fileName: string;
	relativePath: string;
	bytes: number;
	xCount: number;
}

export interface BackupSummary {
	path: string;
	fileName: string;
	createdAt: number;
	sizeBytes: number;
	kind: BackupKind;
	/** False for the "Back up workbooks now" folder, which carries no database. */
	includesDatabase: boolean;
	workbookCount: number;
	totalXCount: number;
}

export interface BackupStatus {
	localBackupDir: string;
	backupCount: number;
	retentionLimit: number;
	lastBackupAt?: number;
	lastBackupPath?: string;
	lastError?: string;
	/** Most recent workbook-only backup, or undefined if none has been taken. */
	lastWorkbooksBackupPath?: string;
}

export interface BackupPreview {
	/** The folder (or legacy file) the user picked. */
	sourcePath: string;
	/** The `attendance.db` actually opened — `sourcePath` for a legacy flat backup. */
	databasePath: string;
	fileName: string;
	modifiedAt: number;
	sizeBytes: number;
	schemaVersion: number;
	studentCount: number;
	classCount: number;
	eventCount: number;
	absentCount: number;
	settingsCount: number;
	sf2TemplateCount: number;
	includesDatabase: boolean;
	/** Empty for a legacy flat backup written before workbooks were backed up. */
	workbooks: BackupWorkbookPreview[];
	warnings: string[];
}

export interface RestoreResult {
	restoredPath: string;
	preRestoreBackupPath: string;
	restoredAt: number;
	schemaVersion: number;
	migrated: boolean;
	workbooksRestored: boolean;
	warnings: string[];
}

/**
 * What "wipe all data" destroyed, and where the safety copy it takes first
 * landed. `preWipeBackupPath` is null when the backup could not be written.
 */
export interface WipeOutcome {
	deletedStudents: number;
	deletedClasses: number;
	deletedEvents: number;
	preWipeBackupPath: string | null;
}

/**
 * The result of writing a class's SF2 workbook metadata.
 *
 * Named for the import that introduced it and kept for `update_sf2_workbook_settings`,
 * which the Reports page still calls. The import flow itself is gone (spec D18).
 */
export interface Sf2ImportSummary {
	templateId: string;
	classId: string;
	className: string;
	sourcePath: string;
	schoolYear: string;
	gradeLevel: string;
	section: string;
	learnersFound: number;
	studentsCreated: number;
	studentsReused: number;
	datesMapped: number;
}

export interface Sf2TemplateDraft {
	classId?: string;
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	firstSchoolDay?: number;
	learnerNames: string[];
}

export interface Sf2WorkbookSettings {
	templateId: string;
	classId: string;
	className: string;
	sourcePath: string;
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	firstSchoolDay: number;
	learnerNames: string[];
	datesMapped: number;
}

export interface Sf2CloseDaySummary {
	classId: string;
	date: string;
	presentCount: number;
	absentCount: number;
}

/**
 * Result of reading "X" absence marks back out of the SF2 working workbook.
 * `imported` counts absences newly recorded; `alreadyRecorded` counts workbook
 * marks the database already knew about, so re-running is a safe no-op.
 *
 * The manual command behind this is no longer reachable from the UI - the
 * startup self-heal does the same job unattended (spec §8, D6) and reports
 * through [`Sf2HealOutcome`]. The shape is kept because the command is still
 * registered, so §12.4's "keep and re-wire" applies to it too.
 */
export interface Sf2AttendanceImportOutcome {
	classId: string;
	reportMonth: string;
	scannedCells: number;
	imported: number;
	alreadyRecorded: number;
	datesWithMarks: number;
	mappedRows: number;
	mappedDates: number;
}

export interface Sf2TemplateSummary {
	id: string;
	sourcePath: string;
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	classId: string;
	importedAt: number;
}

export interface Sf2ExportReadiness {
	template?: Sf2TemplateSummary;
	mappedStudents: number;
	mappedDates: number;
	canExport: boolean;
	issues: string[];
	warnings: string[];
}

export type Sf2PreviewCellStatus = 'present' | 'absent' | 'open';

export interface Sf2ExportPreview {
	template?: Sf2TemplateSummary;
	classId?: string;
	className: string;
	sourcePath?: string;
	dates: Sf2PreviewDate[];
	students: Sf2PreviewStudentRow[];
	absentList: Sf2PreviewAbsence[];
	mappedStudents: number;
	mappedDates: number;
	presentCount: number;
	absenceCount: number;
	unmappedStudentCount: number;
	canExport: boolean;
	issues: string[];
	warnings: string[];
}

export interface Sf2PreviewDate {
	date: string;
	sheetName: string;
	columnLetter: string;
	columnIndex: number;
}

export interface Sf2PreviewStudentRow {
	studentId: string;
	studentName: string;
	workbookName: string;
	gender?: string;
	rowIndex: number;
	mapped: boolean;
	presentCount: number;
	absentCount: number;
	warnings: string[];
	cells: Sf2PreviewCell[];
}

export interface Sf2PreviewCell {
	date: string;
	status: Sf2PreviewCellStatus;
	editable: boolean;
}

export interface Sf2PreviewAbsence {
	studentId: string;
	studentName: string;
	date: string;
	rowIndex: number;
}

export interface Sf2ExportResult {
	outputPath: string;
	marksWritten: number;
}

/**
 * One month workbook: the `.xls` file for a single month of a school year
 * (`SF2-SEPTEMBER-2026.xls`, one worksheet named "SEPTEMBER 2026"), and the row
 * that describes it. One of these per month, twelve per school year.
 */
export interface Sf2MonthTemplate {
	id: string;
	classId: string;
	/** The school year label, e.g. `2026-2027`. */
	schoolYear: string;
	/** Canonical uppercase month name, e.g. `SEPTEMBER`. */
	reportMonth: string;
	/**
	 * The calendar year this month falls in, which is not always the first year
	 * of the school year: SEPTEMBER 2026 - AUGUST 2027 is one school year.
	 */
	reportYear: number;
	sourcePath: string;
	sourceHash: string;
	schoolId?: string;
	schoolName?: string;
	gradeLevel?: string;
	section?: string;
	adviserName?: string;
	schoolHeadName?: string;
	/**
	 * The effective first attendance day, derived from `schoolStartDate` or typed
	 * by the user. `0` means the month is not dated yet - never a guessed day.
	 */
	firstSchoolDay: number;
	/** Set when `firstSchoolDay` was typed by the user; re-derivation skips it. */
	firstSchoolDayOverride?: number;
	importedAt: number;
	/** X marks last counted in the file. Meaningless unless `workbookScannedAt` is set. */
	workbookXCount: number;
	/**
	 * When `workbookXCount` was measured, and when attendance was last written to
	 * the file. Explicitly nullable rather than optional-with-`undefined`: serde
	 * emits `null` for a Rust `Option`, and treating "absent" and "null" as
	 * different things is how a rendered `undefined` gets into a list a teacher
	 * reads. A null `workbookScannedAt` means the file is *unmeasured*, not empty.
	 */
	workbookScannedAt?: number | null;
	lastSyncedAt?: number | null;
}

/**
 * One row of the Settings → Month workbooks list: the stored month plus what
 * the file on disk actually looks like right now. A month with no stored row
 * still appears, so the list always shows all twelve.
 */
export interface Sf2MonthPreview {
	month: string;
	reportYear: number;
	schoolYear: string;
	/** The file this month resolves to, e.g. `SF2-SEPTEMBER-2026.xls`. */
	fileName: string;
	/** A stored row whose file is gone: every write path must refuse, nothing clears. */
	fileExists: boolean;
	hasTemplate: boolean;
	firstSchoolDay: number;
	firstSchoolDayOverridden: boolean;
	workbookXCount: number;
	workbookScannedAt?: number | null;
	lastSyncedAt?: number | null;
	learnerCount: number;
	mappedDateCount: number;
}

/**
 * One month, read from SQL. This is the whole month switch (spec D9, §7.1):
 * `getSf2MonthPreview` returns it, the grid renders it, and nothing in between
 * opens Excel or writes anything.
 *
 * The grid half deliberately mirrors `Sf2ExportPreview` field for field, so
 * the table, the absent list and the sidebar read both without knowing which
 * command produced them. The month half - which file this grid belongs to, and
 * whether that file is there - is what a per-month model can answer and a
 * single workbook with a mutable `reportMonth` could not.
 */
export interface Sf2MonthGridPreview {
	/** Canonical uppercase month name, e.g. `SEPTEMBER`. */
	month: string;
	/**
	 * The calendar year this month falls in, which is not always the first year
	 * of the school year: SEPTEMBER 2026 - AUGUST 2027 is one school year.
	 *
	 * Read off the stored month row, never recomputed in the browser. The legacy
	 * year rule wraps at June and the school-year rule wraps at September, and
	 * they disagree about August; letting the browser pick a year is how a month
	 * ends up filed under the wrong one.
	 */
	reportYear: number;
	schoolYear: string;
	classId: string;
	className: string;
	/** The month's one worksheet, `"{MONTH} {year}"`. Empty when no row exists. */
	sheetName: string;
	/** `SF2-SEPTEMBER-2026.xls`. */
	fileName: string;
	/**
	 * A stored row whose file is gone is edge case E4: every write path refuses
	 * and nothing is cleared. The read still succeeds so the grid can show what
	 * the database holds while the file is restored.
	 */
	fileExists: boolean;
	hasTemplate: boolean;
	/** `0` means the month is not dated yet - never a guessed day. */
	firstSchoolDay: number;
	/**
	 * False for a month with no school days (edge case E2: April, May, summer),
	 * and false while `schoolStartDate` is unset. Never a guess in either
	 * direction.
	 */
	hasSchoolDays: boolean;
	/** No day columns are mapped, so nothing may be written to this month yet. */
	gridEmpty: boolean;
	/**
	 * At least one of the two mapping sets was read from the pre-split
	 * `sf2_date_mappings` / `sf2_student_mappings` rather than from this month's
	 * own per-month row.
	 *
	 * A read state only: nothing is written, and those tables are never modified
	 * or deleted - on the affected install they are still where some of the data
	 * lives. Surfaced so the page can say where the grid is drawn from, rather
	 * than implying the month has no data.
	 */
	usesLegacyMappings: boolean;
	/**
	 * X marks last counted in the file, and when they were counted. The guard's
	 * own comparison, surfaced so the sidebar can show it as information (spec
	 * §12.2). Meaningless unless `workbookScannedAt` is set: a file nobody has
	 * counted is unmeasured, not empty.
	 */
	workbookXCount: number;
	workbookScannedAt?: number;
	/** When attendance was last written to this month's file. */
	lastSyncedAt?: number;
	/**
	 * The month's workbook identity, in the shape the Reports page has always
	 * read for a template. Every month file of a class carries the same school,
	 * grade, section and adviser, so the sidebar's identity panel is fed from
	 * here and never has to reach for a different month's row.
	 */
	template?: Sf2TemplateSummary;
	dates: Sf2PreviewDate[];
	students: Sf2PreviewStudentRow[];
	absentList: Sf2PreviewAbsence[];
	mappedStudents: number;
	mappedDates: number;
	presentCount: number;
	absenceCount: number;
	unmappedStudentCount: number;
	issues: string[];
	warnings: string[];
}

/**
 * Which month the app opens on launch, and whether a create may be offered
 * (spec D5, acceptance #12, edge cases E1 and E2). Read-only.
 */
export interface Sf2LaunchMonth {
	/** The month to open, already resolved through the D5 fallback. */
	month: string;
	reportYear: number;
	schoolYear: string;
	classId: string;
	fileName: string;
	fileExists: boolean;
	hasTemplate: boolean;
	/**
	 * False for a month with no school days, and false while the real start date
	 * has not been entered. A create is never offered for such a month.
	 */
	hasSchoolDays: boolean;
	/** Today's calendar month, before the fallback. The E1 toast names this one. */
	todayMonth: string;
	todayReportYear: number;
	/**
	 * Today's month has no file and the fallback was used, so the user is being
	 * shown a month they did not ask for. The fallback is never silent.
	 */
	fellBack: boolean;
	/** A one-click create may be offered for the month the app opened on. */
	canCreate: boolean;
	/**
	 * Whether a create may be offered for **today's** month. This is the E1 case,
	 * and it is a different question from `canCreate` whenever the fallback ran:
	 * the app is showing May because June has no file, and the thing to offer is
	 * "Create June to switch" - asking only about May would answer for a month
	 * that is already on record.
	 */
	todayCanCreate: boolean;
	/** True while the real start date has not been entered (edge case E3). */
	needsSchoolStartDate: boolean;
	issues: string[];
}

// ── The startup self-heal (spec §8.2, acceptance #15) ──────────────────────

/**
 * What one run of the startup self-heal did (spec §8.2, §8.3).
 *
 * Every variant is a *report*, not an error: a workbook the app cannot read is a
 * state the app handles, and §9.1's guard refuses a write on it at the next Open
 * or Export. The four "nothing happened" variants are the overwhelming majority
 * of launches, which is exactly why
 * [`sf2HealToast`]($lib/features/settings/sf2-heal-toast) returns nothing for
 * them - a toast on every launch is noise, and noise is how a teacher learns to
 * dismiss messages from this app without reading them.
 *
 * Rust side: `sf2::heal::Sf2HealOutcome`, `#[serde(tag = "status",
 * rename_all = "camelCase")]`, so the variant names are camelCased on the wire
 * and every field is camelCased too.
 */
export type Sf2HealOutcome =
	/** This launch's single run already happened (D8). */
	| { status: 'alreadyRan' }
	/** No `sf2_month_templates` row for the resolved month - nothing to measure. */
	| { status: 'notApplicable'; reason: string }
	/** Excel could not read the file: no Excel, locked, or an unexpected layout. */
	| { status: 'excelUnavailable'; reason: string }
	/** The month row exists but the file is not on disk (edge case E4). */
	| { status: 'workbookMissing'; reason: string }
	/** `db_count > workbook_count`. The database is ahead of the file. */
	| {
			status: 'upToDate';
			month: string;
			dbCount: number;
			workbookCount: number;
			scannedAt: number;
	  }
	/** `db_count == workbook_count`. Measured and recorded; nothing imported. */
	| {
			status: 'inSync';
			month: string;
			dbCount: number;
			workbookCount: number;
			scannedAt: number;
	  }
	/** The file held marks the database did not, and they were recorded. */
	| {
			status: 'recovered';
			month: string;
			/** The month's own `SF2-SEPTEMBER-2026.xls` - the file they came from. */
			fileName: string;
			/** Absences actually written. */
			imported: number;
			/** Workbook `X` the database already recorded - the idempotent no-ops. */
			alreadyRecorded: number;
			workbookCount: number;
			dbCountBefore: number;
			dbCountAfter: number;
			scannedAt: number;
			/** §8.2 step 6, ready for the frontend to hand to a toast verbatim. */
			toast: string;
	  };

// ── The workbook merge (spec §11, §12.1) ───────────────────────────────────

/**
 * How one month of the merge went. `verified` was built and proved now;
 * `alreadyMerged` was left alone because the file may hold marks written since;
 * `needsAttention` means the legacy workbook is still the authority for that
 * month.
 */
export type Sf2SplitMonthStatus = 'verified' | 'alreadyMerged' | 'needsAttention';

/** One month of the school year, as the Settings screen renders it. */
export interface Sf2SplitMonthOutcome {
	/** Canonical uppercase month name, e.g. `SEPTEMBER`. */
	reportMonth: string;
	reportYear: number;
	/**
	 * The one worksheet this month lives on, e.g. `SEPTEMBER 2026`. Empty for a
	 * month that could not be built.
	 */
	sheetName: string;
	/**
	 * The one file all twelve months share, e.g.
	 * `SF2-GRADE-3-MATAPAT-3b635890.xls`.
	 *
	 * The same value on all twelve rows. That sameness is the point of the
	 * one-file model, so the screen shows the one name rather than twelve
	 * lookalike ones.
	 */
	fileName: string;
	status: Sf2SplitMonthStatus;
	/** How many `X` marks this month's worksheet holds. */
	xMarks: number;
	learnerRows: number;
	/** Why a month needs attention. Absent for a month that is fine. */
	detail?: string | null;
}

/** The whole merge, as the Settings screen renders it. */
export interface Sf2SplitOutcome {
	/**
	 * When all twelve months verified, or `null` while any still needs attention.
	 * The same value as `settings.sf2_split_completed_at` - which is what makes a
	 * partial run a resume rather than a second, conflicting merge (E12).
	 */
	splitCompletedAt?: number | null;
	months: Sf2SplitMonthOutcome[];
	verifiedCount: number;
	needsAttentionCount: number;
	/** The one workbook all twelve months live in. */
	workbookPath: string;
	/** Where the untouched original lives. */
	legacyFilePath: string;
	/** The D13 workbooks backup taken before anything was written, when one was. */
	legacyBackupPath?: string | null;
	/**
	 * Absences the database holds in months this school year has no worksheet
	 * for.
	 *
	 * Never silently zero when it is not: they are in the database and in the
	 * reports grid, and a complete-looking file that quietly omits nine of a
	 * teacher's absences is how data appears to vanish.
	 */
	absencesOutsideSchoolYear: number;
	/** The sentence the Settings screen shows, worded by the backend. */
	message: string;
}

// ── The school calendar settings (spec D16, §11.1) ─────────────────────────

/**
 * The two v22 settings the per-month model depends on.
 *
 * `schoolStartDate` is `null` and stays `null` until the user types it. There is
 * deliberately no default: a guessed start date silently mis-dates every month
 * file in the school year, which is the exact class of bug this whole model
 * exists to eliminate. The app prompts once instead (E3).
 */
export interface Sf2SchoolCalendarSettings {
	/** `YYYY-MM-DD`, or `null` when it has never been entered. */
	schoolStartDate: string | null;
	/** The month the app falls back on when today's month has no file (D5). */
	lastReportMonth: string | null;
}
