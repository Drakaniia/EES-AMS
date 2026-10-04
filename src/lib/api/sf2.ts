/**
 * The SF2 workbook half of `$lib/api/sf2.ts`.
 *
 * ## The month is resolved, never invented
 *
 * Rust had one workbook per class with a mutable `reportMonth` column, so every
 * command took just a `classId` and the row decided the month. Under the per-month
 * model a month is a worksheet in a shared file, and the row that answers "which
 * month" is the launch month (spec D5). Commands whose old signature carries no
 * month therefore resolve one here; `syncAndOpenSf2Workbook` is the exception,
 * because it already took `reportMonth` and passing it through unchanged is what
 * stops "Open SF2" from syncing one month and opening another.
 *
 * ## No Excel process (spec D14)
 *
 * Writing is ExcelJS; opening is `@tauri-apps/plugin-opener`. `killAllExcelProcesses`
 * exists only as a throwing stub so the one live call site keeps compiling while its
 * button is deleted — see the handover table.
 */

import { openPath } from '@tauri-apps/plugin-opener';
import { appError, asAppError, internal, invalidInput } from '$lib/db';
import { getClass } from '$lib/db/repos/classes';
import { listStudents } from '$lib/db/repos/students';
import { nowEpochSeconds } from '$lib/domain/models';
import { getExportsDir } from '$lib/features/backup/paths';
import {
	guardPermitsMonthRewrite,
	presentAllPreviewAttendance,
	setPreviewAttendanceLightweight,
	syncAndOpenSf2Workbook as writeAndOpenWorkbook
} from '$lib/features/sf2/attendance/attendance-service';
import type { Sf2OpenResult } from '$lib/features/sf2/attendance/attendance-service';
import {
	monthAbsences,
	resolveMonthWriteContext
} from '$lib/features/sf2/attendance/write-context';
import { writeAttendanceToWorkbook } from '$lib/features/sf2/attendance/attendance-write';
import { getSf2LaunchMonth, getSf2MonthPreview } from '$lib/features/sf2/month/month';
import {
	countMonthDateMappings,
	findMonthTemplate,
	setMonthLastSyncedAt
} from '$lib/features/sf2/month/templates';
import { monthRosterForTemplate } from '$lib/features/sf2/month/students';
import type { Sf2ProgressReporter } from '$lib/features/sf2/progress';
import { createWorkbookFromTemplate } from '$lib/features/sf2/template/create';
import {
	importSf2WorkbookFromFile,
	stageImportSource,
	validateSf2WorkbookImport
} from '$lib/features/sf2/template/import';
import type { Sf2ImportValidation } from '$lib/features/sf2/validation';
import { updateWorkbookSettings } from '$lib/features/sf2/template/update';
import { exportWorkbookFileName } from '$lib/features/sf2/workbook-files';
import { getFileSystem } from '$lib/platform/fs';
import type {
	Sf2ExportPreview,
	Sf2ExportReadiness,
	Sf2ExportResult,
	Sf2ImportSummary,
	Sf2TemplateDraft,
	Sf2WorkbookSettings
} from '$lib/types';

export type {
	Sf2ExportPreview,
	Sf2ExportReadiness,
	Sf2ExportResult,
	Sf2ImportSummary,
	Sf2PreviewCell,
	Sf2PreviewStudentRow,
	Sf2TemplateDraft,
	Sf2WorkbookSettings
} from '$lib/types';

export type { Sf2ImportValidation } from '$lib/features/sf2/validation';

/** The month a `classId`-only command means: today's, else the last used (spec D5). */
async function launchMonth(classId?: string) {
	return await getSf2LaunchMonth(classId);
}

/** `canExport` is the grid's one derived verdict, so it is derived here too. */
function withCanExport<T extends { issues: string[] }>(grid: T): T & { canExport: boolean } {
	return { ...grid, canExport: grid.issues.length === 0 };
}

/** The launch month's grid, with the export verdict added. */
async function launchGrid(
	classId?: string
): Promise<Awaited<ReturnType<typeof getSf2MonthPreview>> & { canExport: boolean }> {
	const launch = await launchMonth(classId);
	return withCanExport(await getSf2MonthPreview(launch.month, launch.classId, launch.schoolYear));
}

// â”€â”€ Workbook identity â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/**
 * The workbook details the Settings dialog hydrates from.
 *
 * Read off the launch month's own row rather than the retired `sf2_templates`
 * table: on a fully migrated install that table is empty, and a settings dialog
 * filled in from an empty table is a dialog the teacher cannot save.
 */
export async function getSf2WorkbookSettings(classId?: string): Promise<Sf2WorkbookSettings> {
	const launch = await launchMonth(classId);
	const month = await findMonthTemplate(launch.classId, launch.schoolYear, launch.month);
	if (month === undefined) {
		throw invalidInput('No SF2 workbook has been set up for this class yet.');
	}

	const [schoolClass, roster, datesMapped] = await Promise.all([
		getClass(launch.classId),
		monthRosterForTemplate(month.id),
		countMonthDateMappings(month.id)
	]);

	return {
		templateId: month.id,
		classId: launch.classId,
		className: schoolClass?.name ?? '',
		sourcePath: month.sourcePath,
		schoolId: month.schoolId ?? '',
		schoolName: month.schoolName ?? '',
		schoolYear: month.schoolYear,
		reportMonth: month.reportMonth,
		gradeLevel: month.gradeLevel ?? '',
		section: month.section ?? '',
		adviserName: month.adviserName ?? '',
		schoolHeadName: month.schoolHeadName ?? '',
		firstSchoolDay: month.firstSchoolDay,
		// Row 0 is the "no workbook row" placeholder, never a learner on the form.
		learnerNames: roster.filter((row) => row.rowIndex > 0).map((row) => row.workbookName),
		datesMapped
	};
}

export async function updateSf2WorkbookSettings(
	draft: Sf2TemplateDraft
): Promise<Sf2ImportSummary> {
	return await updateWorkbookSettings(draft);
}

/** Create a class workbook from the bundled DepEd template. */
export async function createSf2WorkbookFromTemplate(
	draft: Sf2TemplateDraft
): Promise<Sf2ImportSummary> {
	return await createWorkbookFromTemplate(draft);
}

/**
 * One workbook — the pick-then-act flow `open_sf2_workbook` always had.
 *
 * `null` when the teacher dismissed the picker, which is not an error.
 * Lazily imported: `plugin-dialog` exists only inside a shell.
 */
export async function pickImportWorkbookFile(): Promise<string | null> {
	const { pickWorkbookFile } = await import('./pickers');
	return await pickWorkbookFile();
}

/** Copy a picked file somewhere the fs allow-list covers (see `stageImportSource`). */
export async function stageImportWorkbook(pickedPath: string): Promise<string> {
	return await stageImportSource(pickedPath);
}

/** The validation report for the import dialog; analyzes without writing anything. */
export async function validateSf2WorkbookImportFile(
	stagedPath: string
): Promise<Sf2ImportValidation> {
	return await validateSf2WorkbookImport(stagedPath);
}

/** Adopt a staged workbook after (explicit-proceed) validation. */
export async function importSf2Workbook(
	stagedPath: string,
	proceedAnyway: boolean
): Promise<Sf2ImportSummary> {
	return await importSf2WorkbookFromFile(stagedPath, proceedAnyway);
}

// â”€â”€ The grid â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

export async function getSf2ExportReadiness(classId?: string): Promise<Sf2ExportReadiness> {
	const grid = await launchGrid(classId);

	return {
		template: grid.template,
		mappedStudents: grid.mappedStudents,
		mappedDates: grid.mappedDates,
		canExport: grid.canExport,
		issues: grid.issues,
		warnings: grid.warnings
	};
}

/**
 * `Sf2MonthGridPreview` is `Sf2ExportPreview` plus the month half, so the export
 * preview is the grid with the one derived field added — no second builder, and
 * therefore no second answer to "what does an X mean".
 */
export async function getSf2ExportPreview(classId?: string): Promise<Sf2ExportPreview> {
	return await launchGrid(classId);
}

// â”€â”€ Writing attendance â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/**
 * Push the database's absences into one month's worksheet and stamp the sync.
 * Returns the working-copy path and how many marks the write produced.
 */
async function writeMonth(
	classId: string,
	reportMonth: string,
	progress?: Sf2ProgressReporter
): Promise<{ sourcePath: string; marksWritten: number }> {
	const context = await resolveMonthWriteContext(classId, reportMonth);
	const schoolClass = await getClass(classId);
	if (schoolClass === undefined) throw appError('ClassNotFound', classId);

	const absent = await monthAbsences(context, await listStudents(classId));
	const marksWritten = await writeAttendanceToWorkbook({
		target: {
			sourcePath: context.sourcePath,
			sheetName: context.sheetName,
			roster: context.roster,
			dates: context.dates
		},
		absentIdsFor: (date) => absent.get(date) ?? new Set<string>(),
		progress
	});
	await setMonthLastSyncedAt(context.templateId, nowEpochSeconds());

	return { sourcePath: context.sourcePath, marksWritten };
}

/**
 * Sync the latest attendance events from the database to the SF2 working copy.
 *
 * Background sync: the database is already correct, so a guard that does not
 * return `Proven` means log and skip the write, never fail the attendance
 * action that triggered the sync.
 */
export async function syncSf2Attendance(classId: string): Promise<void> {
	const launch = await launchMonth(classId);
	if (!(await guardPermitsMonthRewrite({ classId: launch.classId, reportMonth: launch.month }))) {
		console.warn(
			`skipping SF2 attendance sync for ${launch.classId}: the guard did not return Proven`
		);
		return;
	}
	await writeMonth(launch.classId, launch.month);
}

/** Lightweight toggle — only writes the DB event, no Excel I/O or preview rebuild. */
export async function toggleSf2PreviewAttendance(
	classId: string,
	studentId: string,
	date: string,
	present: boolean
): Promise<void> {
	await setPreviewAttendanceLightweight({ classId, studentId, date, present });
}

/** The same write, followed by the rebuilt preview the old command returned. */
export async function setSf2PreviewAttendance(
	classId: string,
	studentId: string,
	date: string,
	present: boolean
): Promise<Sf2ExportPreview> {
	await toggleSf2PreviewAttendance(classId, studentId, date, present);
	return await getSf2ExportPreview(classId);
}

/**
 * Mark every mapped learner present for the launch month, and report how many
 * absences were cleared. Open days are left as they are.
 */
export async function presentAllSf2PreviewAttendance(classId: string): Promise<number> {
	const launch = await launchMonth(classId);
	return await presentAllPreviewAttendance({ classId: launch.classId, reportMonth: launch.month });
}

/**
 * Sync attendance into the month's worksheet, then hand the file to the OS.
 *
 * Returns the file with the month it holds, so the UI names what was actually
 * opened rather than echoing a path.
 *
 * `progress` is the new seam: the Rust command emitted ten `sf2-progress` Tauri
 * events and the frontend listened. It is now a callback, so
 * `report-sf2-open.svelte.ts` can drop its `listen()` and pass one. Left optional
 * so the existing two-argument call site keeps compiling until it is rewired.
 */
export async function syncAndOpenSf2Workbook(
	classId: string,
	reportMonth: string,
	progress?: Sf2ProgressReporter
): Promise<Sf2OpenResult> {
	const opened = await writeAndOpenWorkbook({ classId, reportMonth, progress });
	await openWithOs(opened.path);
	return opened;
}

// â”€â”€ Export and open â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/**
 * "Export SF2": write the month's absences into the working copy, copy that file
 * into `Documents\EES-AMS\exports` under its own name, and open the copy — so the
 * working copy stays writable while the teacher has a dated snapshot.
 */
export async function exportSf2Workbook(classId: string): Promise<Sf2ExportResult> {
	const launch = await launchMonth(classId);
	const { sourcePath, marksWritten } = await writeMonth(launch.classId, launch.month);

	const month = await findMonthTemplate(launch.classId, launch.schoolYear, launch.month);
	const outputPath = `${await getExportsDir()}/${exportWorkbookFileName(
		month?.gradeLevel ?? '',
		month?.section ?? '',
		launch.month,
		new Date().getMonth() + 1
	)}`;
	await getFileSystem().writeFileAtomic(outputPath, await getFileSystem().readFile(sourcePath));
	await openWithOs(outputPath);

	return { outputPath, marksWritten };
}

/**
 * Pick a workbook and open it — the pick-then-act flow `open_sf2_workbook` had.
 *
 * The empty string is a dismissed picker, not an error, and the type has always
 * said `Promise<string>`. `./pickers` is imported lazily because it reaches for
 * `@tauri-apps/plugin-dialog`, which only exists inside a shell.
 */
export async function openSf2Workbook(_classId?: string): Promise<string> {
	const { pickWorkbookFile } = await import('./pickers');
	const picked = await pickWorkbookFile();
	if (picked === null) return '';
	await openWithOs(picked);
	return picked;
}

async function openWithOs(path: string): Promise<void> {
	try {
		await openPath(path);
	} catch (thrown) {
		throw internal(`failed to open ${path}: ${asAppError(thrown).detail}`);
	}
}

/**
 * D14 removed the Excel process this killed. The export is kept as a throwing stub
 * rather than an omission so the single live call site
 * (`report-sf2-open.svelte.ts:278`, the "Kill Excel and retry" button) keeps
 * compiling while that button is deleted; the throw is the loudest possible signal
 * that the retry it powers no longer exists.
 */
export async function killAllExcelProcesses(): Promise<number> {
	throw invalidInput('Excel is no longer driven by this app');
}
