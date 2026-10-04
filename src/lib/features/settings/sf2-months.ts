import type { Sf2MonthPreview, Sf2SplitOutcome } from '$lib/types';

/**
 * Pure helpers for the Settings → SF2 Workbook screen (spec §12.1).
 *
 * Everything that can be decided without a command lives here, so it can be
 * tested in a plain Vitest process: this repository's test environment has no
 * `document`, and a screen whose only logic is "sort twelve rows and word two
 * labels" does not need one.
 */

/**
 * The school year in running order, SEPTEMBER → AUGUST (spec D2).
 *
 * Not alphabetical and not the calendar year: a school year starts in September,
 * and a list sorted any other way puts JANUARY next to SEPTEMBER and reads as
 * though the year starts in summer.
 */
export const SF2_SCHOOL_YEAR_ORDER = [
	'SEPTEMBER',
	'OCTOBER',
	'NOVEMBER',
	'DECEMBER',
	'JANUARY',
	'FEBRUARY',
	'MARCH',
	'APRIL',
	'MAY',
	'JUNE',
	'JULY',
	'AUGUST'
] as const;

/** One row of the *Month workbooks* list, ready to render. */
export type Sf2MonthWorkbookRow = {
	month: string;
	/** `September 2026` - the month and the year it actually falls in. */
	label: string;
	fileName: string;
	/** `Present` / `Missing` / `Not set up`, as the file is right now. */
	fileState: string;
	/** `ready` when the month is set up and its file is there. */
	tone: 'ready' | 'attention' | 'pending';
	/**
	 * X marks last counted in the file, or `null` when nobody has counted it. A
	 * file nobody has measured is *unmeasured*, not empty - and nothing may be
	 * cleared on a guess of zero - so "not measured" is its own display state.
	 */
	xCount: number | null;
	/** `Never`, a date, or `Not measured yet` when the count is unknown. */
	lastSynced: string;
	firstSchoolDay: number;
	/** True when the teacher typed this month's first day by hand. */
	firstSchoolDayOverridden: boolean;
	/** 0 means the month is undated - never a guessed day. */
	undated: boolean;
};

const MONTH_LABELS: Record<string, string> = {
	JANUARY: 'January',
	FEBRUARY: 'February',
	MARCH: 'March',
	APRIL: 'April',
	MAY: 'May',
	JUNE: 'June',
	JULY: 'July',
	AUGUST: 'August',
	SEPTEMBER: 'September',
	OCTOBER: 'October',
	NOVEMBER: 'November',
	DECEMBER: 'December'
};

export function sf2MonthLabel(month: string): string {
	return MONTH_LABELS[month.toUpperCase()] ?? month.trim();
}

function orderOf(month: string): number {
	const index = SF2_SCHOOL_YEAR_ORDER.indexOf(month.toUpperCase() as never);
	// A month the backend names that this list has never heard of sorts to the
	// end rather than disappearing: an unknown month is still a month on disk.
	return index === -1 ? SF2_SCHOOL_YEAR_ORDER.length : index;
}

function formatUnixSeconds(value?: number | null): string | null {
	if (!value) return null;
	const date = new Date(value * 1000);
	if (Number.isNaN(date.getTime())) return null;
	return date.toLocaleString(undefined, {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: 'numeric',
		minute: '2-digit'
	});
}

/** Build one renderable row from a backend row. */
function sf2MonthWorkbookRow(row: Sf2MonthPreview): Sf2MonthWorkbookRow {
	const month = row.month.toUpperCase();
	const measured = typeof row.workbookScannedAt === 'number' && row.workbookScannedAt > 0;

	let fileState: string;
	let tone: Sf2MonthWorkbookRow['tone'];
	if (!row.hasTemplate) {
		fileState = 'Not set up';
		tone = 'pending';
	} else if (!row.fileExists) {
		// Edge case E4: a stored row whose file is gone. Every write path refuses
		// on this state and the guard stays unmeasured, so it is worth flagging.
		fileState = 'Missing';
		tone = 'attention';
	} else {
		fileState = 'Present';
		tone = 'ready';
	}

	return {
		month,
		label: `${sf2MonthLabel(month)} ${row.reportYear}`,
		fileName: row.fileName,
		fileState,
		tone,
		xCount: measured ? row.workbookXCount : null,
		lastSynced: formatUnixSeconds(row.lastSyncedAt) ?? 'Never',
		firstSchoolDay: row.firstSchoolDay,
		firstSchoolDayOverridden: row.firstSchoolDayOverridden,
		undated: row.firstSchoolDay === 0
	};
}

/** Sort twelve rows into school-year order. Does not mutate the input. */
export function sf2MonthWorkbookRows(rows: Sf2MonthPreview[]): Sf2MonthWorkbookRow[] {
	return [...rows]
		.map(sf2MonthWorkbookRow)
		.sort((left, right) => orderOf(left.month) - orderOf(right.month));
}

/** The one-line state of the whole year, under the list. */
export function sf2MonthWorkbookSummary(rows: Sf2MonthWorkbookRow[]): string {
	if (rows.length === 0) return 'No SF2 month workbooks yet.';

	const ready = rows.filter((row) => row.tone === 'ready').length;
	const missing = rows.filter((row) => row.tone === 'attention').length;
	const measured = rows.filter((row) => row.xCount !== null).length;
	const totalX = rows.reduce((sum, row) => sum + (row.xCount ?? 0), 0);

	const parts = [`${ready} of ${rows.length} month files ready`];
	if (missing > 0) parts.push(`${missing} missing from disk`);
	parts.push(`${measured} measured · ${totalX} X mark${totalX === 1 ? '' : 's'} counted`);
	return parts.join(' · ');
}

// ── "Classes started on" (spec D16, §11.1, edge case E3) ───────────────────

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

/**
 * Turn a typed date into the `YYYY-MM-DD` the setting stores, or `null`.
 *
 * Returns `null` for blank and for anything that is not a real calendar date, so
 * "cleared" and "typed something wrong" both land as `null` and the field keeps
 * whatever was there rather than writing a value nobody meant. It never returns a
 * default: an unset start date is a state the app handles by dating months
 * from June, and a guessed one is the
 * bug this whole model exists to prevent.
 */
export function normalizeSchoolStartDate(value: string): string | null {
	const trimmed = value.trim();
	if (!trimmed) return null;
	if (!ISO_DATE.test(trimmed)) return null;

	const [year, month, day] = trimmed.split('-').map(Number);
	if (year === undefined || month === undefined || day === undefined) return null;
	// Round-tripping through `Date` is the check that catches 2026-02-31, which
	// the shape of the string cannot.
	const parsed = new Date(Date.UTC(year, month - 1, day));
	if (
		parsed.getUTCFullYear() !== year ||
		parsed.getUTCMonth() !== month - 1 ||
		parsed.getUTCDate() !== day
	) {
		return null;
	}
	return trimmed;
}

/** Is what is in the date box worth saving? */
export function isSchoolStartDateValid(value: string): boolean {
	return value.trim() === '' || normalizeSchoolStartDate(value) !== null;
}

// ── The merge result (spec §11) ─────────────────────────────────────────────

/**
 * One line describing what a merge run did, preferring the backend's own wording.
 *
 * Named for the command it reports on (`run_sf2_workbook_split`, kept so the
 * Settings screen's wire call and the `sf2_split_completed_at` setting that
 * already exist in the field both keep working), but what it describes is a
 * rebuild of **one** workbook holding twelve month sheets - not twelve files.
 */
export function sf2SplitSummary(outcome: Sf2SplitOutcome): string {
	if (outcome.message.trim()) return outcome.message.trim();
	return sf2SplitSummaryFromCounts(outcome.verifiedCount, outcome.needsAttentionCount);
}

function sf2SplitSummaryFromCounts(verified: number, needsAttention: number): string {
	// "The original workbook is kept" is the promise that makes the whole operation
	// safe to run without reading the spec, so it is in both sentences rather than
	// only the cheerful one.
	if (needsAttention === 0) {
		return `All ${verified} months are ready in one workbook. The original workbook is kept in sf2-workbooks_legacy.`;
	}
	return `${verified} months are ready, ${needsAttention} need attention. The original workbook is kept in sf2-workbooks_legacy.`;
}

/** The months a human still has to do something about, named the way the report does. */
export function sf2SplitNeedsAttention(outcome: Sf2SplitOutcome): string[] {
	return outcome.months
		.filter((month) => month.status === 'needsAttention')
		.map((month) => `${sf2MonthLabel(month.reportMonth)} ${month.reportYear}`);
}

/** Has every one of the twelve months verified? Mirrors Rust's `is_merge_complete`. */
export function sf2SplitIsComplete(outcome: Sf2SplitOutcome): boolean {
	return outcome.splitCompletedAt !== null && outcome.splitCompletedAt !== undefined;
}
