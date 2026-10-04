/**
 * Deriving a month's first attendance day (spec D16, §11.1) — the port of
 * `src-tauri/src/sf2/month/first_school_day.rs` and the resolution half of
 * `month/merge.rs`.
 *
 * ```text
 * first_school_day(MONTH, YEAR) =
 *     the first Mon-Fri day on or after max(school_start_date, first day of MONTH)
 *     clamped into MONTH
 * ```
 *
 * Everything here is pure: no IO, no database and no clock, except
 * {@link currentYear}, which is named rather than inlined so the "we do not know"
 * case is visible at every call site. The caller supplies the calendar year, so
 * the whole derivation is a function of its arguments — which matters, because a
 * wrong day here silently mis-dates a month file.
 *
 * ## `school_start_date` is not defaulted
 *
 * There is no "typical Philippine school year" fallback here. An unset
 * `school_start_date` stays unset and {@link deriveFirstSchoolDay} returns
 * `undefined`; the caller falls back to the legacy per-month value and prompts
 * once ({@link needsSchoolStartDatePrompt}, spec edge case E3). A guessed default
 * would be wrong for every school that does not start on that Monday, and nothing
 * downstream would notice.
 */

import { normalizeSchoolYear } from '$lib/db/migrations';
import { normalizeSchoolStartDate } from '$lib/features/settings/sf2-months';

/**
 * The first month of the school year, September.
 *
 * A Philippine school year runs SEPTEMBER -> AUGUST, so the school year
 * `2026-2027` covers SEPTEMBER 2026 through AUGUST 2027. Months from September on
 * belong to the start year; the rest belong to the following calendar year. The
 * v22 backfill applies the same rule in SQL (`migrate_to_v22.sql`).
 */
export const SCHOOL_YEAR_START_MONTH = 9;

/** Shown once, when the real start date has not been entered yet (spec E3). */
export const SCHOOL_START_DATE_PROMPT =
	"Enter the date classes started so each month's SF2 can be dated automatically.";

/**
 * What the `first_school_day` column holds when nothing is known.
 *
 * The v22 backfill has no dates to derive from, so it writes `0` rather than a
 * guess. Every read of the column must route through {@link knownFirstSchoolDay}:
 * treating `0` as a day would date a month from a day that never happened, and
 * writing one back would make the guess permanent.
 */
export const FIRST_SCHOOL_DAY_UNDETERMINED = 0;

// ── Dates ────────────────────────────────────────────────────────────────────

/**
 * A calendar date at UTC midnight — the stand-in for Rust's `NaiveDate`.
 *
 * UTC because a `NaiveDate` has no timezone, and the local-timezone `Date` would
 * move a date across a day boundary for half the world. `undefined` for a day the
 * month does not have, which is what `NaiveDate::from_ymd_opt` answered.
 */
export function naiveDate(year: number, month: number, day: number): Date | undefined {
	const date = new Date(Date.UTC(year, month - 1, day));
	const roundTrips =
		date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day;
	return roundTrips ? date : undefined;
}

/**
 * Parse a stored `YYYY-MM-DD`, or `undefined` for anything that is not a real
 * calendar date.
 *
 * Rust's `parse_date` returned `Result<NaiveDate>`, but every caller of it in the
 * SF2 code discarded the error and treated an unparseable date as absent, so the
 * honest port is `undefined` rather than a throw nobody would catch. The
 * validation itself is the app's one ISO-date validator, not a second one.
 */
export function parseIsoDate(value: string): Date | undefined {
	const normalized = normalizeSchoolStartDate(value);
	if (normalized === null) return undefined;
	const [year, month, day] = normalized.split('-').map(Number);
	return naiveDate(year, month, day);
}

/** The `YYYY-MM-DD` a date is stored as. */
export function formatIsoDate(date: Date): string {
	const year = String(date.getUTCFullYear()).padStart(4, '0');
	const month = String(date.getUTCMonth() + 1).padStart(2, '0');
	const day = String(date.getUTCDate()).padStart(2, '0');
	return `${year}-${month}-${day}`;
}

/** Add whole days to a UTC-midnight date. DST-safe, because the date is in UTC. */
export function addDays(date: Date, days: number): Date {
	return new Date(date.getTime() + days * 86_400_000);
}

/**
 * The last calendar day of a month.
 *
 * Day 0 of the next month, less one — the only definition that needs no table of
 * month lengths to be right for February in a leap year.
 */
export function lastDayOfMonth(year: number, month: number): number {
	const firstOfNextMonth = month === 12 ? naiveDate(year + 1, 1, 1) : naiveDate(year, month + 1, 1);
	return firstOfNextMonth === undefined ? 31 : addDays(firstOfNextMonth, -1).getUTCDate();
}

// ── The school year ──────────────────────────────────────────────────────────

/**
 * Every four-digit year in a label, in the order they appear, keeping only the
 * plausible ones (1900-2999) and the first of each repeated value.
 *
 * Parsed from {@link normalizeSchoolYear}'s canonical form, which is what makes
 * the two spellings of one school year (`2026-2027` and `2026 - 2027`) resolve
 * identically here and in the SQL that looks the months up.
 */
export function schoolYearYears(schoolYear: string): number[] {
	const years: number[] = [];
	for (const part of normalizeSchoolYear(schoolYear).split(/[^0-9]/)) {
		if (part.length !== 4) continue;
		const year = Number(part);
		if (year < 1900 || year > 2999 || years.includes(year)) continue;
		years.push(year);
	}
	return years;
}

/** The first four-digit year in a school-year label, e.g. `2026` for `2026-2027`. */
export function schoolYearStartYear(schoolYear: string): number | undefined {
	return schoolYearYears(schoolYear)[0];
}

/**
 * Which calendar year a month of the school year falls in.
 *
 * `fallbackYear` is used when `schoolYear` holds no four-digit year. The fallback
 * is a parameter rather than a hidden `new Date()` read so this function stays
 * pure and the caller decides what "we do not know" means.
 */
export function reportYearForSchoolMonth(
	schoolYear: string,
	reportMonth: number,
	fallbackYear: number
): number {
	const startYear = schoolYearStartYear(schoolYear);
	if (startYear === undefined) return fallbackYear;
	return reportMonth >= SCHOOL_YEAR_START_MONTH ? startYear : startYear + 1;
}

/**
 * The calendar year to fall back on when a school year holds no year.
 *
 * The one impure thing in the month model, kept behind a named function so
 * {@link reportYearForSchoolMonth} itself stays a pure function of its arguments.
 */
export function currentYear(): number {
	return new Date().getFullYear();
}

// ── School days ──────────────────────────────────────────────────────────────

/**
 * Monday to Friday. A Philippine school week is five days; Saturday and Sunday are
 * never attendance days.
 */
export function isSchoolDay(date: Date): boolean {
	const weekday = date.getUTCDay();
	return weekday >= 1 && weekday <= 5;
}

/**
 * The first attendance day of a month, or `undefined` when it cannot be known.
 *
 * `undefined` means one of exactly two things, and the caller has to tell them
 * apart with {@link needsSchoolStartDatePrompt}:
 *
 * * `schoolStartDate` is unset — the user has not been asked yet.
 * * Classes start after the month ends, so the month has no school days at all
 *   (spec edge case E2: never auto-create a file for such a month).
 */
export function deriveFirstSchoolDay(
	schoolStartDate: Date | undefined,
	reportMonth: number,
	reportYear: number
): number | undefined {
	// Unset means unset. See the module docs.
	if (schoolStartDate === undefined) return undefined;

	const firstOfMonth = naiveDate(reportYear, reportMonth, 1);
	const lastOfMonth = naiveDate(reportYear, reportMonth, lastDayOfMonth(reportYear, reportMonth));
	if (firstOfMonth === undefined || lastOfMonth === undefined) return undefined;

	// A month before classes started is dated from its own first school day; a
	// month classes started in the middle of is dated from the start date.
	let candidate =
		firstOfMonth.getTime() >= schoolStartDate.getTime() ? firstOfMonth : schoolStartDate;
	if (candidate.getTime() > lastOfMonth.getTime()) return undefined;

	// First Monday-Friday on or after the candidate, still inside the month.
	while (!isSchoolDay(candidate)) {
		candidate = addDays(candidate, 1);
		if (candidate.getTime() > lastOfMonth.getTime()) return undefined;
	}

	// "Clamped into MONTH" holds by construction; keep the check so a future change
	// to the arithmetic cannot leak a day from the next month.
	if (candidate.getUTCMonth() + 1 !== reportMonth || candidate.getUTCFullYear() !== reportYear) {
		return undefined;
	}
	return candidate.getUTCDate();
}

/** Should the app ask for the real start date? True exactly while it is unset. */
export function needsSchoolStartDatePrompt(schoolStartDate: Date | undefined): boolean {
	return schoolStartDate === undefined;
}

/**
 * Drop the "not derived yet" sentinel.
 *
 * The column is `NOT NULL` because every reader and the workbook writer need one
 * effective number with no branching, so "unknown" is stored as `0` rather than
 * `NULL`. Reading that `0` as a day is the bug this function exists to stop.
 */
export function knownFirstSchoolDay(day: number | undefined): number | undefined {
	return day === undefined || day === FIRST_SCHOOL_DAY_UNDETERMINED ? undefined : day;
}

/**
 * Resolve a month file's first attendance day from what is stored on its row.
 *
 * An override always wins. Re-derivation is free to run as often as it likes — a
 * change to `schoolStartDate`, a re-run of the split — and must never discard a
 * day the user typed (spec §11.1). The legacy day the pre-split workbook recorded
 * is the last resort, and only when it is a real day rather than the sentinel.
 */
export function effectiveFirstSchoolDay(
	overridden: number | undefined,
	derived: number | undefined
): number | undefined {
	return knownFirstSchoolDay(overridden) ?? derived;
}

export function resolveFirstSchoolDay(
	schoolStartDate: Date | undefined,
	reportMonth: number,
	reportYear: number,
	overrideDay: number | undefined,
	legacyDay: number | undefined
): number | undefined {
	return effectiveFirstSchoolDay(
		overrideDay,
		deriveFirstSchoolDay(schoolStartDate, reportMonth, reportYear) ?? knownFirstSchoolDay(legacyDay)
	);
}

/**
 * The day the day grid is anchored on, which is not the same as the month's first
 * attendance day.
 *
 * The month row still stores {@link FIRST_SCHOOL_DAY_UNDETERMINED} when no day is
 * known, so the Settings list shows the month as undated, the prompt for *Classes
 * started on* still fires once, and nothing anywhere claims a school day that was
 * never established. What the grid says is "these are the days of this month", not
 * "class started on the 1st" — so an undated month still gets a full grid.
 */
export function gridAnchorDay(
	firstSchoolDay: number | undefined,
	reportYear: number,
	reportMonth: number
): number {
	const day = knownFirstSchoolDay(firstSchoolDay);
	const lastDay = lastDayOfMonth(reportYear, reportMonth);
	return day !== undefined && day >= 1 && day <= lastDay ? day : 1;
}
