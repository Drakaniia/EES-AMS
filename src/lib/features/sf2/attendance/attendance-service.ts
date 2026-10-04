/**
 * The SF2 attendance service — the port of
 * `src-tauri/src/sf2/attendance/attendance_service.rs`.
 *
 * ## The guard
 *
 * `run_write_guard` / `SyncPermit` / `SyncAction` live in `../guard.ts`, adapted
 * from COM to ExcelJS: measuring is a cell-text read off the opened workbook,
 * and the read-only branch skips all writes and hands the path to the OS opener
 * normally (there is no COM `ReadOnly:=True`; the guarantee is "the app never
 * writes"). The differential clear in `attendance-marks.ts` remains the second,
 * structural layer, as in Rust.
 *
 * ## No Excel process
 *
 * A write is ExcelJS plus a temp-file rename; opening the file is the caller's
 * job with the opener plugin (spec D14). The ten-step progress vocabulary
 * survives in `progress.ts`, because that is the UI's vocabulary and not
 * Excel's.
 *
 * ## The month is a parameter, with a latest-template fallback
 *
 * `reportMonth` decides three separate things: whose absences are written, whose
 * day columns they land in, and which worksheet is made active. A month with no
 * row falls back to the class's newest month row (what the retired
 * `latest_template_for_class` would have opened), and only then to a pre-split
 * row. Syncing one month and opening another is a bug, so the resolved month —
 * not the asked one — drives the whole write.
 */

import { appError, getDriver } from '$lib/db';
import { getClass } from '$lib/db/repos/classes';
import { listStudents } from '$lib/db/repos/students';
import type { Workbook } from 'exceljs';
import {
	getCellTextAt,
	openWorkbook,
	saveWorkbookAtomic,
	sf2MonthlySheets
} from '$lib/features/excel/workbook';
import { bestSf2MonthlySheet } from '$lib/features/sf2/calendar';
import { getFileSystem } from '$lib/platform/fs';
import { NO_PROGRESS, emitSf2Progress } from '$lib/features/sf2/progress';
import type { Sf2ProgressReporter } from '$lib/features/sf2/progress';
import { nowEpochSeconds } from '$lib/domain/models';
import type { Student } from '$lib/domain/models';
import type { Sf2AttendanceImportOutcome } from '$lib/types';
import { classWorkbookFiles } from '$lib/features/sf2/workbook-files';
import { clearLastSyncedAtForClass, setMonthLastSyncedAt } from '$lib/features/sf2/month/templates';
import { setLastSyncedAt as setLegacyLastSyncedAt } from '$lib/features/sf2/repository';
import { recordAuditEvent } from '$lib/db/repos/audit';
import {
	SF2_PRESENT_ALL_CORRECTION,
	SF2_PREVIEW_CORRECTION,
	attendanceTimestampForDate,
	localDayBoundsTimestampsForDate,
	setAttendanceEventForDay
} from './attendance-events';
import { emptyImportOutcome, importAbsentMarks } from './attendance-import';
import { monthAbsences, monthPresences, resolveMonthWriteContext } from './write-context';
import type { Sf2MonthWriteContext } from './write-context';
import { exportAttendanceMarks, writeOpenMonthIntoWorkbook } from './attendance-write';
import { attendanceScopeCells, gridCellFromMark, type Sf2GridCell } from './attendance-marks';
import {
	cellLabels,
	decide,
	guardBeforeWrite,
	measureWorkbookMarks,
	WORKBOOK_NOT_READABLE,
	type SyncAction,
	type SyncPermit
} from '../guard';

/** Shown when the open is asked for with no month on screen. */
export const NO_MONTH_SELECTED_MESSAGE =
	'No month is selected. Switch to a month before opening the SF2 workbook.';

/** Shown when the workbook the month row names is not on disk (edge case E4). */
export const WORKBOOK_MISSING_MESSAGE =
	'The app SF2 working workbook no longer exists. Import the SF2 workbook again';

/**
 * What opening an SF2 workbook resolved to: the file, and the month it holds.
 *
 * The month is the resolved one — what the fallback derived when the asked
 * month had no row — because that is the sheet the write activated. Reporting
 * the asked month would claim a sheet the file may not even hold.
 */
export type Sf2OpenResult = {
	/** The working-copy path the caller should open. */
	path: string;
	reportMonth: string;
	reportYear: number;
};

function openedResult(context: Sf2MonthWriteContext): Sf2OpenResult {
	return {
		path: context.sourcePath,
		reportMonth: context.reportMonth,
		reportYear: context.reportYear
	};
}

/**
 * Refuse to open when the class has more than one workbook file on disk.
 *
 * A re-import or a retried split can orphan the previous working copy beside
 * the current one (same `SF2-GRADE-SECTION-` stem, different id suffix), and
 * the month rows name only one of them. Opening either silently is how the
 * teacher ends up editing the stale copy, so this names every candidate and
 * stops: keep the file the database names, remove the rest, open again.
 */
async function requireSingleClassWorkbook(context: Sf2MonthWriteContext): Promise<void> {
	const { gradeLevel, section } = context.layout.metadata;
	const directory = context.sourcePath.split(/[\\/]/).slice(0, -1).join('/');
	const files = await classWorkbookFiles(directory, gradeLevel, section);
	if (files.length <= 1) return;
	const names = files.map((file) => file.split(/[\\/]/).pop() ?? file);
	const kept = context.sourcePath.split(/[\\/]/).pop() ?? context.sourcePath;
	throw appError(
		'InvalidInput',
		`Two SF2 workbooks exist for ${gradeLevel} ${section} (${names.join(', ')}). ` +
			`The app opens ${kept}. Remove the other file(s) from the workbooks folder and open again.`
	);
}

/** The class's roster and its `day_start`, or an error naming what is missing. */
async function classRoster(classId: string): Promise<{ students: Student[]; dayStart: string }> {
	const cls = await getClass(classId);
	if (cls === undefined) throw appError('InvalidInput', 'Selected class was not found');
	return { students: await listStudents(classId), dayStart: cls.dayStart };
}

// ── The service ───────────────────────────────────────────────────────────────

/**
 * Push the latest attendance events into the workbook for the month on screen, and
 * hand back the file to open with the month it holds.
 *
 * Ten progress steps, the same numbers and messages the Rust emitted — including
 * the guard's `Checking the workbook against the app…`.
 *
 * The guard runs before anything is written. `ReadOnly` skips all writes and the
 * sync stamp and returns the file for a normal open; `Aborted` refuses without
 * touching the file or the stamp. There is deliberately no "has anything changed
 * since the last sync" shortcut on top of the guard: `Proven` already proves the
 * database holds every mark the workbook shows, so writing is always safe from
 * there, and `last_synced_at` is purely a record of the last write.
 */
export async function syncAndOpenSf2Workbook(params: {
	classId: string;
	reportMonth: string;
	progress?: Sf2ProgressReporter;
}): Promise<Sf2OpenResult> {
	const { classId, reportMonth, progress = NO_PROGRESS } = params;
	emitSf2Progress(progress, 1);

	const asked = reportMonth.trim();
	if (asked === '') throw appError('InvalidInput', NO_MONTH_SELECTED_MESSAGE);
	// `true`: a month with no mapped days reaches the guard below, which reads it
	// as `Unmeasured` and opens read-only, exactly as Rust did. Refusing here
	// would also put a refusal ahead of the missing-workbook check, which Rust
	// reached only from inside that read-only branch.
	const context = await resolveMonthWriteContext(classId, asked, undefined, true);
	emitSf2Progress(progress, 2, 'Reading student data…');
	emitSf2Progress(progress, 3, 'Checking date mappings…');

	if (!(await getFileSystem().exists(context.sourcePath))) {
		throw appError('InvalidInput', WORKBOOK_MISSING_MESSAGE);
	}

	// One class, one file: a stale orphan beside the working copy is a refusal,
	// never a guess.
	await requireSingleClassWorkbook(context);

	const { students, dayStart } = await classRoster(classId);

	// The destructive-sync guard. Nothing below this line may clear the grid
	// until the database has proven it holds the workbook's marks.
	emitSf2Progress(progress, 3, 'Checking the workbook against the app…');
	const guarded = await runOpenGuard({ classId, context, students, dayStart });
	const { action } = guarded;
	const workbook = guarded.workbook;

	if (action.kind === 'ReadOnly') {
		// Unmeasured: the workbook could not be measured, so the app never
		// writes. The user still sees their marks, through a normal open.
		emitSf2Progress(progress, 4, `Opening read-only: ${action.reason}`);
		emitSf2Progress(progress, 10);
		return openedResult(context);
	}
	if (action.kind === 'Aborted') {
		emitSf2Progress(progress, 4, 'Workbook not in sync…');
		throw appError('InvalidInput', action.message);
	}

	emitSf2Progress(progress, 4, 'Clearing previous marks…');
	emitSf2Progress(progress, 5, 'Computing attendance marks…');

	const absent = guarded.absent;
	emitSf2Progress(progress, 6, 'Writing marks to workbook…');
	const opened = workbook ?? (await openWorkbook(context.sourcePath).catch(() => undefined));
	if (opened === undefined) throw appError('InvalidInput', WORKBOOK_MISSING_MESSAGE);
	writeOpenMonthIntoWorkbook({
		workbook: opened,
		target: {
			sourcePath: context.sourcePath,
			sheetName: context.sheetName,
			roster: context.roster,
			dates: context.dates
		},
		month: { reportMonth: context.reportMonth, reportYear: context.reportYear },
		layout: context.layout,
		absentIdsFor: (date) => absent.get(date) ?? new Set<string>(),
		progress
	});

	emitSf2Progress(progress, 7, 'Saving workbook changes…');
	await saveWorkbookAtomic(opened, context.sourcePath);
	await stampSync(context, nowEpochSeconds());

	// Steps 8-10 are the handover. The write above set the synced month as the
	// active tab, so the file reopens on it rather than on whatever tab the last
	// write left in front.
	emitSf2Progress(progress, 8, 'Preparing to open…');
	if (!(await getFileSystem().exists(context.sourcePath))) {
		throw appError('InvalidInput', WORKBOOK_MISSING_MESSAGE);
	}
	emitSf2Progress(progress, 9, 'Opening in Microsoft Excel…');
	emitSf2Progress(progress, 10);
	return openedResult(context);
}

/**
 * Whether a non-open write may rewrite the month's grid.
 *
 * The background sync's guard: anything but `Rewrite` means log and skip, never
 * fail the attendance action that triggered the sync.
 */
export async function guardPermitsMonthRewrite(params: {
	classId: string;
	reportMonth: string;
}): Promise<boolean> {
	const { classId, reportMonth } = params;
	const context = await resolveMonthWriteContext(classId, reportMonth.trim());
	if (!(await getFileSystem().exists(context.sourcePath))) return false;
	const { students, dayStart } = await classRoster(classId);
	const { action } = await runOpenGuard({ classId, context, students, dayStart });
	return action.kind === 'Rewrite';
}

type OpenGuard = {
	action: SyncAction;
	workbook: Workbook | undefined;
	absent: Map<string, Set<string>>;
};

/**
 * Evaluate the guard for an open: measure the workbook against the database,
 * import-then-recheck at most once, and hand back the verdict with the workbook
 * and the post-import absences the write needs.
 */
async function runOpenGuard(params: {
	classId: string;
	context: Sf2MonthWriteContext;
	students: Student[];
	dayStart: string;
}): Promise<OpenGuard> {
	const { classId, context, students, dayStart } = params;
	const workbook = await openWorkbookOrUndefined(context.sourcePath);
	const measureName = measureSheetName(workbook, context.sheetName);
	const loadAbsences = () => monthAbsences(context, students);
	const evaluate = async (): Promise<SyncPermit> => {
		if (workbook === undefined || measureName === undefined) {
			return { kind: 'Unmeasured', reason: WORKBOOK_NOT_READABLE };
		}
		const dates = context.dates.map((date) => ({ ...date, sheetName: measureName }));
		const scope = attendanceScopeCells(context.roster, dates);
		const measured = measureWorkbookMarks(workbook, context.roster, dates, scope);
		if (!measured.ok) return { kind: 'Unmeasured', reason: measured.reason };
		return decide(
			dbCellsFor(context, dates, await loadAbsences()),
			measured.xCells,
			cellLabels(context.roster, dates),
			WORKBOOK_NOT_READABLE,
			await forgivenCells(context, dates, students)
		);
	};

	const action = await guardBeforeWrite(evaluate, async () => {
		if (workbook === undefined || measureName === undefined) return;
		const dates = context.dates.map((date) => ({ ...date, sheetName: measureName }));
		await importAbsentMarks({
			classId,
			reportMonth: context.reportMonth,
			dayStart,
			roster: context.roster,
			dates,
			textFor: (sheetName, address) => {
				const sheet = workbook.getWorksheet(sheetName);
				return sheet === undefined ? undefined : getCellTextAt(sheet, address);
			}
		});
	});

	return { action, workbook, absent: await loadAbsences() };
}

/** The database's `X` cells over remapped dates, for the guard to compare. */
function dbCellsFor(
	context: Sf2MonthWriteContext,
	dates: Sf2MonthWriteContext['dates'],
	absent: Map<string, Set<string>>
): Sf2GridCell[] {
	return exportAttendanceMarks({
		dates,
		roster: context.roster,
		absentIdsFor: (date) => absent.get(date) ?? new Set<string>()
	})
		.map(gridCellFromMark)
		.filter((cell): cell is Sf2GridCell => cell !== undefined);
}

/**
 * Scope cells the database explicitly marks present, keyed like the guard's.
 *
 * A workbook `X` on one of these is a stale mark from before the teacher's
 * correction — the rewrite clears it instead of the import resurrecting the
 * absence (which is what deleted the correction and restored the `X`).
 */
async function forgivenCells(
	context: Sf2MonthWriteContext,
	dates: Sf2MonthWriteContext['dates'],
	students: Student[]
): Promise<Set<string>> {
	const present = await monthPresences(context, students);
	const studentByRow = new Map(
		context.roster
			.filter((mapping) => mapping.rowIndex > 0)
			.map((mapping) => [mapping.rowIndex, mapping.studentId])
	);
	const forgiven = new Set<string>();
	for (const date of dates) {
		const ids = present.get(date.date);
		if (ids === undefined) continue;
		for (const [rowIndex, studentId] of studentByRow) {
			if (ids.has(studentId))
				forgiven.add(`${date.sheetName ?? ''}!${date.columnLetter}${rowIndex}`);
		}
	}
	return forgiven;
}

/** Open a workbook, or `undefined` when it cannot be read (guard: Unmeasured). */
async function openWorkbookOrUndefined(path: string): Promise<Workbook | undefined> {
	try {
		return await openWorkbook(path);
	} catch {
		return undefined;
	}
}

/**
 * The sheet the guard measures: the target, or — for a single-sheet workbook
 * or a school's own variant naming — the most populated monthly sheet, without
 * renaming anything yet (renaming is a write, and the guard may refuse writes).
 */
function measureSheetName(workbook: Workbook | undefined, targetName: string): string | undefined {
	if (workbook === undefined) return undefined;
	const monthlySheets = sf2MonthlySheets(workbook);
	const sheet =
		monthlySheets.find((candidate) => candidate.name === targetName) ??
		bestSf2MonthlySheet(monthlySheets) ??
		monthlySheets[0];
	return sheet?.name;
}

/** Record the sync where the resolved row lives: the month table, or the legacy one. */
async function stampSync(context: Sf2MonthWriteContext, syncedAt: number): Promise<void> {
	if (context.legacyTemplateId !== undefined) {
		await setLegacyLastSyncedAt(context.legacyTemplateId, syncedAt);
		return;
	}
	await setMonthLastSyncedAt(context.templateId, syncedAt);
}

/**
 * Mark one learner's grid cell, without touching the workbook.
 *
 * The fast path behind the grid's click-to-correct: the database is the source of
 * truth, so persisting the event *is* the whole operation. The workbook catches up
 * on the next open, and every month of the class is marked unsynced so that open
 * actually happens - the corrected mark carries a past date, so without the reset a
 * last-write comparison would call the month in sync and skip the rewrite.
 *
 * No month row is required. The Rust demanded one and answered "No SF2 template
 * imported for this class" on a fully migrated install, which made a grid's cells
 * look clickable and then fail on click - a worse failure than a disabled cell,
 * because the teacher is told the mark was saved and it was not.
 *
 * The date mapping is deliberately *not* required either: an unmapped day is still
 * clickable and still recorded. A day the grid has no column for is filtered out at
 * export time, which is where that knowledge belongs.
 */
export async function setPreviewAttendanceLightweight(params: {
	classId: string;
	studentId: string;
	date: string;
	present: boolean;
}): Promise<void> {
	const { classId, studentId, date, present } = params;
	const { students, dayStart } = await classRoster(classId);
	if (!students.some((student) => student.id === studentId)) {
		throw appError('InvalidInput', 'Selected student was not found');
	}

	await setAttendanceEventForDay({
		studentId,
		classId,
		date,
		dayStart,
		eventType: present ? 'in' : 'absent',
		reason: SF2_PREVIEW_CORRECTION
	});

	// A grid correction writes to `events`, which belongs to no month, so every month
	// of the class is stale the moment it lands. Scoped by class for exactly that.
	await clearLastSyncedAtForClass(classId);
}

/**
 * Mark every learner present for the month on screen, and report how many
 * absences were cleared.
 *
 * Each cleared absence becomes an explicit `in` record — the same record
 * unmarking one grid cell writes — because a deleted event leaves no trace and
 * the open guard would resurrect the workbook's `X` over the silence. Days
 * with no absence to clear get no record: present-by-default needs none.
 *
 * Open days are left alone, so a teacher's note in a column the app has no record
 * for survives.
 */
export async function presentAllPreviewAttendance(params: {
	classId: string;
	reportMonth: string;
}): Promise<number> {
	const { classId, reportMonth } = params;
	const context = await resolveMonthWriteContext(classId, reportMonth);
	const { students, dayStart } = await classRoster(classId);
	const rosterIds = new Set(students.map((student) => student.id));
	const driver = getDriver();

	let deleted = 0;
	const cleared: { studentId: string; date: string }[] = [];
	const seen = new Set<string>();
	await driver.transaction(async () => {
		for (const date of context.dates) {
			const { start, end } = localDayBoundsTimestampsForDate(date.date);
			const rows = await driver.query<{ id: string; student_id: string; class_id: string | null }>(
				`SELECT id, student_id, class_id FROM events
				 WHERE event_type = 'absent' AND timestamp >= ? AND timestamp < ?`,
				[start, end]
			);
			for (const row of rows) {
				// An event with no class counts when the learner is on the roster: that
				// is how a mark taken while another class was selected is stored.
				if (row.class_id !== classId && !rosterIds.has(row.student_id)) continue;
				await driver.execute('DELETE FROM events WHERE id = ?', [row.id]);
				deleted += 1;
				const key = `${row.student_id}|${date.date}`;
				if (!seen.has(key)) {
					seen.add(key);
					cleared.push({ studentId: row.student_id, date: date.date });
				}
			}
		}
		if (cleared.length > 0) {
			for (const { studentId, date } of cleared) {
				const { start, end } = localDayBoundsTimestampsForDate(date);
				await driver.execute(
					`DELETE FROM events
					 WHERE student_id = ?
					   AND timestamp >= ?
					   AND timestamp < ?
					   AND (class_id IS NULL OR class_id = ?)`,
					[studentId, start, end, classId]
				);
				await driver.execute(
					`INSERT INTO events
					   (id, student_id, class_id, event_type, timestamp, note, session_key, override_reason, updated_at)
					 VALUES (?, ?, ?, 'in', ?, ?, ?, ?, NULL)`,
					[
						crypto.randomUUID(),
						studentId,
						classId,
						attendanceTimestampForDate(date, dayStart),
						SF2_PRESENT_ALL_CORRECTION,
						`${date}|${classId}|day`,
						SF2_PRESENT_ALL_CORRECTION
					]
				);
			}
			await recordAuditEvent({
				entityType: 'attendance_event',
				action: 'create',
				summary: `Marked ${cleared.length} learner-day(s) present for ${context.reportMonth} (${SF2_PRESENT_ALL_CORRECTION})`,
				metadataJson: JSON.stringify({
					classId,
					reportMonth: context.reportMonth,
					eventType: 'in',
					clearedAbsences: deleted,
					presentRecords: cleared.length,
					reason: SF2_PRESENT_ALL_CORRECTION
				})
			});
		}
	});

	if (deleted > 0) await clearLastSyncedAtForClass(classId);
	return deleted;
}

/**
 * Read the `X` marks back out of the workbook for the month on screen and record
 * them as absences.
 *
 * Additive and idempotent: an `X` becomes an event only when the database does not
 * already record that learner absent for that day, so re-running is a no-op rather
 * than a duplicate. Every month of the class is marked unsynced when anything was
 * imported, so the next open rewrites the grid from the now-complete database
 * instead of skipping.
 */
export async function importAbsentMarksFromWorkbook(params: {
	classId: string;
	reportMonth: string;
}): Promise<Sf2AttendanceImportOutcome> {
	const { classId, reportMonth } = params;
	const context = await resolveMonthWriteContext(classId, reportMonth);
	if (context.roster.length === 0) {
		return emptyImportOutcome(classId, context.reportMonth, context.dates.length);
	}
	if (!(await getFileSystem().exists(context.sourcePath))) {
		throw appError('InvalidInput', WORKBOOK_MISSING_MESSAGE);
	}

	const workbook = await openWorkbook(context.sourcePath);
	const outcome = await importAbsentMarks({
		classId,
		reportMonth: context.reportMonth,
		dayStart: (await classRoster(classId)).dayStart,
		roster: context.roster,
		dates: context.dates,
		textFor: (sheetName, address) => {
			const sheet = workbook.getWorksheet(sheetName);
			return sheet === undefined ? undefined : getCellTextAt(sheet, address);
		}
	});

	if (outcome.imported > 0) await clearLastSyncedAtForClass(classId);
	return outcome;
}
