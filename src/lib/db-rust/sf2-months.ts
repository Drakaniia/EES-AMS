import { invoke } from '@tauri-apps/api/core';
import type {
	Sf2LaunchMonth,
	Sf2MonthGridPreview,
	Sf2MonthPreview,
	Sf2MonthTemplate,
	Sf2SchoolCalendarSettings,
	Sf2SplitOutcome
} from '../types';
export type {
	Sf2LaunchMonth,
	Sf2MonthGridPreview,
	Sf2MonthPreview,
	Sf2MonthTemplate,
	Sf2SchoolCalendarSettings,
	Sf2SplitOutcome
} from '../types';

/**
 * Read one month of the SF2 reports. This is the whole month switch.
 *
 * A read-only SQL command: no Excel, no COM, no write, and no `sf2-progress`
 * event, so there is nothing for a modal to cover. The Reports page caches the
 * result per `${classId}:${schoolYear}:${month}`, so switching back to a month
 * that has already been read costs nothing at all.
 *
 * `classId` and `schoolYear` are both optional. Left out, the backend resolves
 * the only class on record and the school year that class actually has months
 * for - which is what a fresh launch, before anything has been chosen, needs.
 */
export async function getSf2MonthPreview(
	reportMonth: string,
	classId?: string,
	schoolYear?: string
): Promise<Sf2MonthGridPreview> {
	return await invoke('get_sf2_month_preview', {
		classId: classId || null,
		schoolYear: schoolYear || null,
		reportMonth
	});
}

/**
 * Which month to open on launch: today's calendar month, falling back to the
 * last-used month when today's has no file (spec D5, acceptance #12).
 *
 * The fallback is never silent - `fellBack` is set and both months are named -
 * and `canCreate` is false for a month with no school days (edge case E2).
 */
export async function getSf2LaunchMonth(classId?: string): Promise<Sf2LaunchMonth> {
	return await invoke('get_sf2_launch_month', { classId: classId || null });
}

/**
 * Create the file for one month, so the user can switch to it in one click
 * (spec edge case E1).
 *
 * This is the only mutating command in the switch path, and it is not part of a
 * switch: it is the button in the E1 banner. It writes a fresh copy of the
 * bundled template, so the new month starts with zero marks - it never clones
 * the previous month's file, because that would put one month's X marks in
 * another. Refuses a month with no school days.
 */
export async function createSf2MonthFile(
	reportMonth: string,
	classId?: string
): Promise<Sf2MonthTemplate> {
	return await invoke('create_sf2_month_file', {
		classId: classId || null,
		reportMonth
	});
}

// ── Settings → SF2 Workbook ───────────────────────────────────────────────

/**
 * All twelve month workbooks of the class's school year, as the Settings screen
 * lists them (spec §12.1).
 *
 * Twelve rows, always: a month with no stored row still comes back (as
 * `hasTemplate: false`), because "this month has not been set up yet" and "this
 * month is not part of the year" are different answers and only one of them is
 * ever true. Read-only - no Excel, no writes.
 */
export async function listSf2MonthWorkbooks(classId?: string): Promise<Sf2MonthPreview[]> {
	return await invoke('list_sf2_month_workbooks', { classId: classId || null });
}

/**
 * The two settings the per-month model dates itself from (spec D16, §11.1).
 *
 * `schoolStartDate` is `null` until the user types it, and there is deliberately
 * no default to fall back on: a guessed start date silently mis-dates every
 * month file in the year. `null` is the answer the caller has to handle, not an
 * error.
 */
export async function getSf2SchoolCalendarSettings(): Promise<Sf2SchoolCalendarSettings> {
	return await invoke('get_sf2_school_calendar_settings');
}

/**
 * Record - or clear - the real date classes started.
 *
 * Pass `null` to unset it. Months whose `first_school_day` was overridden by
 * hand keep their override; re-derivation never touches one, so typing this date
 * cannot retro-date a month the teacher already dated themselves.
 */
export async function setSf2SchoolStartDate(schoolStartDate: string | null): Promise<void> {
	await invoke('set_sf2_school_start_date', { schoolStartDate });
}

/**
 * Re-invoke the one-time split of the pre-split workbook (spec §11, D14).
 *
 * There is no `force` flag on purpose, and the split is idempotent: it decides
 * for itself which months still need building, so this is safe to press twice,
 * and a month that is already split is never rebuilt over the marks the app has
 * written into it since. Months that need a human are named in the returned
 * `message`.
 */
export async function runSf2WorkbookSplit(): Promise<Sf2SplitOutcome> {
	return await invoke('run_sf2_workbook_split');
}
