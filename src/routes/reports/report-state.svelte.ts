import { SvelteDate, SvelteMap } from 'svelte/reactivity';
import { errorMessage as appErrorMessage, type AppError } from '$lib/db';
import type { Sf2MonthGridPreview, Sf2PreviewDate, Sf2ExportPreview } from '$lib/types';
import { sf2MonthByValue, sf2ReportMonthLabel } from '$lib/features/settings/sf2-workbook';
import type { Sf2PreviewCell, Sf2PreviewStudentRow } from '$lib/api';

// ── Types ──────────────────────────────────────────────────────────────────────

export const MATRIX_WEEKDAYS = ['M', 'T', 'W', 'TH', 'F'] as const;

type MatrixWeekday = (typeof MATRIX_WEEKDAYS)[number];

export type MatrixDateSlot = {
	key: string;
	weekday: MatrixWeekday;
	date: Sf2PreviewDate | null;
	dateKey: string | null;
	/** True for the Monday slot that opens a week group. */
	weekStart: boolean;
	/** Precomputed for header render — avoids new SvelteDate() per cell per frame. */
	dayNumber: string;
	dateLabel: string;
	title: string;
};

export type MatrixWeekGroup = {
	key: string;
	label: string;
	slots: MatrixDateSlot[];
};

export type MatrixCell = Sf2PreviewCell & {
	/** `${studentId}:${date}` — precomputed so template never calls cellKey(). */
	key: string;
	label: string;
	cls: string;
};

/**
 * A student row flattened for rendering. `cellColumns` is aligned 1:1 with the
 * flat `MatrixDateSlot[]` returned by {@link flattenMatrixSlots}, so the table
 * template resolves a cell with a plain array read instead of hashing a date on
 * every render (`null` = blank slot with no class day). Each cell is enriched
 * with precomputed `key`/`label`/`cls` so the grid hot path does no function
 * calls or SvelteDate allocs.
 */
export type MatrixStudentRow = Omit<Sf2PreviewStudentRow, 'cells'> & {
	cells: Sf2PreviewCell[];
	cellColumns: (MatrixCell | null)[];
};

// ── Pure utility functions ──────────────────────────────────────────────────────

export function errorMessage(error: unknown, fallback: string) {
	if (error instanceof Error) return error.message;
	if (typeof error === 'string') return error;
	// `$lib/db` throws plain `{ kind, detail }` objects, not `Error`s. Without this
	// branch every database failure reaches the teacher as the generic fallback --
	// which is the whole message, so the real reason is never shown.
	if (typeof error === 'object' && error !== null && 'kind' in error && 'detail' in error) {
		return appErrorMessage(error as AppError);
	}
	return fallback;
}

export function formatDate(date: string) {
	const value = new SvelteDate(`${date}T00:00:00`);
	return value.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

function formatWeekday(date: string) {
	const value = new SvelteDate(`${date}T00:00:00`);
	return value.toLocaleDateString(undefined, { weekday: 'short' });
}

function formatDayNumber(date: string) {
	const value = new SvelteDate(`${date}T00:00:00`);
	return String(value.getDate());
}

export function matrixDateLabel(date: string) {
	return `${formatWeekday(date)} ${formatDayNumber(date)}`;
}

export function formatImportedAt(value?: number) {
	if (!value) return 'Not imported';
	return new SvelteDate(value * 1000).toLocaleDateString(undefined, {
		month: 'short',
		day: 'numeric',
		year: 'numeric'
	});
}

// ── The passive workbook status line (spec §12.2) ─────────────────────────────

/**
 * The inputs of the status line, and the only two numbers in it.
 *
 * Both counts are the guard's own (`SyncPermit`'s `workbook_count` and
 * `db_count`), scoped to this month's mapped learner rows � mapped day columns.
 * `workbookXCount` is `null` for a month whose file has never been counted, and
 * that is a third state, not zero: an unmeasured workbook is one the app cannot
 * make any claim about, which is exactly what §9.1's `Unmeasured` default is for.
 */
type WorkbookStatusCounts = {
	/** Unix seconds, or `null` when the file has never been counted. */
	scannedAt: number | null;
	workbookXCount: number | null;
	appXCount: number;
};

type WorkbookStatusLine = {
	/** The sentence, ready to render. */
	text: string;
	/**
	 * `true` when both counts agree, `false` when they do not, `null` when the
	 * workbook was never measured and there is nothing to compare.
	 *
	 * `false` is the state §9.1 exists for: the guard refuses every write, so the
	 * line says so rather than leaving the teacher to find out from a refused
	 * export.
	 */
	agrees: boolean | null;
};

/**
 * "Last checked <time> · workbook has 12 X · app has 12" (spec §12.2).
 *
 * Passive by construction. This function reads two counts and a timestamp and
 * returns a sentence — there is no path from it to a write, and no path from it
 * to a repair. The repair already happened, unattended, at startup; what is left
 * to show is the evidence for it.
 *
 * Written as a pure function rather than inline in the sidebar so the wording is
 * testable, and so the two states a teacher must be able to tell apart — "the
 * workbook has never been read" and "the workbook has been read and holds
 * nothing" — cannot be collapsed into one another by a later edit.
 */
export function workbookStatusLine({
	scannedAt,
	workbookXCount,
	appXCount
}: WorkbookStatusCounts): WorkbookStatusLine {
	if (workbookXCount === null) {
		return {
			text: `${appXCount} X in the app · this month's workbook has not been checked yet`,
			agrees: null
		};
	}
	return {
		text: `Last checked ${formatCheckedAt(scannedAt)} · workbook has ${workbookXCount} X · app has ${appXCount}`,
		agrees: workbookXCount === appXCount
	};
}

/**
 * A time for "last checked".
 *
 * `never` when the count is somehow present without a timestamp, which the
 * backend cannot produce — the two are written together — but which a hand-edited
 * payload or a future field default could, and "Last checked never" is a worse
 * answer than a plain word.
 */
export function formatCheckedAt(value: number | null): string {
	if (!value) return 'never';
	return new SvelteDate(value * 1000).toLocaleString(undefined, {
		month: 'short',
		day: 'numeric',
		hour: 'numeric',
		minute: '2-digit'
	});
}

export function cellKey(studentId: string, date: string) {
	return `${studentId}:${date}`;
}

function cellLabelFor(studentName: string, date: string, status: Sf2PreviewCell['status']) {
	const state = status === 'absent' ? 'absent' : 'present';
	return `${studentName}, ${matrixDateLabel(date)}: ${state}`;
}

function cellClassFor(mapped: boolean, status: Sf2PreviewCell['status']) {
	if (!mapped) return 'border-border bg-surface text-muted-foreground';
	if (status === 'absent') return 'border-red-500/35 bg-red-50 text-red-700';
	return 'border-border bg-background text-muted-foreground';
}

/**
 * Flattens week groups into a single ordered slot list for the grid body. The
 * header still uses the grouped form (for `colspan`); the body iterates the
 * flat list so each row renders one `{#each}` block instead of one per week.
 */
export function flattenMatrixSlots(groups: MatrixWeekGroup[]) {
	const slots: MatrixDateSlot[] = [];
	for (const group of groups) slots.push(...group.slots);
	return slots;
}

/**
 * Projects the preview rows onto the visible slot columns. This runs once per
 * preview load (inside a `$derived`), so the per-render cost of the grid is a
 * plain array read per cell rather than a date-keyed lookup.
 */
export function buildMatrixRows(
	students: Sf2PreviewStudentRow[],
	slots: MatrixDateSlot[],
	genderFilter: 'all' | 'male' | 'female'
): MatrixStudentRow[] {
	const rows: MatrixStudentRow[] = [];

	for (const student of students) {
		if (genderFilter !== 'all' && student.gender?.toLowerCase() !== genderFilter) continue;

		// A plain `Map`, not a `SvelteMap`: this map is a local that dies with the
		// loop, so its reactivity is never observed — and the reactive proxy was
		// ~96% of the cost of a month switch (40 allocations x 22 writes each).
		// eslint-disable-next-line svelte/prefer-svelte-reactivity
		const cellsByDate = new Map<string, Sf2PreviewCell>();
		for (const cell of student.cells) cellsByDate.set(cell.date, cell);

		rows.push({
			...student,
			cellColumns: slots.map((slot) => {
				if (!slot.dateKey) return null;
				const raw = cellsByDate.get(slot.dateKey);
				if (!raw) return null;
				const key = `${student.studentId}:${raw.date}`;
				return {
					...raw,
					key,
					label: cellLabelFor(student.studentName, raw.date, raw.status),
					cls: cellClassFor(student.mapped, raw.status)
				} satisfies MatrixCell;
			})
		});
	}

	return rows;
}

export function reportMonthLabel(value: string) {
	return sf2ReportMonthLabel(value) || 'Blank';
}

/**
 * Project a month read onto the shape the rest of the Reports page already
 * reads.
 *
 * The grid, the absent list, the export dialogs and the sidebar have always
 * consumed `Sf2ExportPreview`. A month read returns the same information, so
 * this is the one place the two meet - which is what keeps "which command
 * produced this" out of the render path: the components cannot tell, and do not
 * need to.
 *
 * `canExport` is deliberately the month's own `issues` rather than a stored
 * flag. It is derived from whether the month is ready, and a stored flag would
 * be a thing that can be true of a month whose file has since gone missing.
 */
export function monthGridToPreview(grid: Sf2MonthGridPreview): Sf2ExportPreview {
	return {
		template: grid.template,
		classId: grid.classId,
		className: grid.className,
		sourcePath: grid.template?.sourcePath ?? grid.fileName,
		dates: grid.dates,
		students: grid.students,
		absentList: grid.absentList,
		mappedStudents: grid.mappedStudents,
		mappedDates: grid.mappedDates,
		presentCount: grid.presentCount,
		absenceCount: grid.absenceCount,
		unmappedStudentCount: grid.unmappedStudentCount,
		canExport: grid.issues.length === 0,
		issues: grid.issues,
		warnings: grid.warnings
	};
}

function createMatrixWeekGroup(key: string): MatrixWeekGroup {
	return {
		key,
		label: '',
		slots: MATRIX_WEEKDAYS.map((weekday) => ({
			key: `${key}-${weekday}`,
			weekday,
			date: null,
			dateKey: null,
			weekStart: weekday === MATRIX_WEEKDAYS[0],
			dayNumber: '',
			dateLabel: '',
			title: `${weekday}, no class day in this month`
		}))
	};
}

function enrichSlot(
	weekday: MatrixWeekday,
	date: Sf2PreviewDate | null,
	dateKey: string | null,
	weekStart: boolean
): MatrixDateSlot {
	const key = dateKey ?? `${weekday}-${weekStart ? 'start' : 'mid'}`;
	if (!dateKey) {
		return {
			key,
			weekday,
			date,
			dateKey,
			weekStart,
			dayNumber: '',
			dateLabel: '',
			title: `${weekday}, no class day in this month`
		};
	}
	const dayNumber = formatDayNumber(dateKey);
	const dateLabel = matrixDateLabel(dateKey);
	const col = date ? ` ${date.columnLetter}${date.columnIndex}` : '';
	return {
		key: dateKey,
		weekday,
		date,
		dateKey,
		weekStart,
		dayNumber,
		dateLabel,
		title: `${dateLabel}${col}`
	};
}

function mondayDateKey(date: string) {
	const [year, month, day] = date.split('-').map(Number);
	const value = new SvelteDate(year, month - 1, day);
	const weekday = value.getDay();
	const mondayOffset = weekday === 0 ? -6 : 1 - weekday;
	return localDateKey(new SvelteDate(year, month - 1, day + mondayOffset));
}

export function weekdayIndexForDate(date: string) {
	const value = new SvelteDate(`${date}T00:00:00`);
	const weekday = value.getDay();
	if (weekday === 0 || weekday === 6) return -1;
	return weekday - 1;
}

function localDateKey(date: Date) {
	const year = date.getFullYear();
	const month = String(date.getMonth() + 1).padStart(2, '0');
	const day = String(date.getDate()).padStart(2, '0');
	return `${year}-${month}-${day}`;
}

/**
 * The calendar year a month belongs to, from the first date that carries one.
 *
 * Only reached when the caller had no `reportYear` to give, which is a month with
 * no dates at all - a month that has not been read. A month that *has* been read
 * arrives with its own year and never comes through here.
 */
function inferReportYear(dates: Sf2PreviewDate[]): number {
	const first = dates[0]?.date;
	const fromFirst = first ? Number.parseInt(first.slice(0, 4), 10) : Number.NaN;
	return Number.isFinite(fromFirst) ? fromFirst : new SvelteDate().getFullYear();
}

/**
 * The weekday columns of one month, grouped into weeks.
 *
 * `reportYear` is the month's own calendar year, as the month read reported it.
 * It is a parameter and not something worked out here, because a school year
 * straddles two calendar years and the two rules that could be used to pick one
 * disagree: the legacy SF2 rule wrapped at **June**, the school-year rule wraps
 * at **September**, and for AUGUST they name different files. The browser is
 * therefore never in the business of deciding - it draws the year the database
 * gave it. Note that this function has no school-year parameter to apply a rule
 * to, which is the point.
 *
 * When it is omitted it falls back to the first mapped date, and only failing
 * that to the current year. Both fallbacks are for a month with no dates at all,
 * which is a month that has not been read.
 */
export function buildMatrixWeekGroups(
	dates: Sf2PreviewDate[],
	reportMonth: string,
	reportYear?: number
): MatrixWeekGroup[] {
	const month = sf2MonthByValue(reportMonth);

	// Pre-index dates by dateKey for O(1) lookup instead of O(n) Array.find per slot
	const datesByKey = new SvelteMap<string, Sf2PreviewDate>();
	for (const d of dates) {
		datesByKey.set(d.date, d);
	}

	if (month) {
		const year = reportYear ?? inferReportYear(dates);
		const dayCount = new SvelteDate(year, month.monthIndex + 1, 0).getDate();
		// Index groups by week key for O(1) lookup
		const groupsByKey = new SvelteMap<string, MatrixWeekGroup>();
		const groups: MatrixWeekGroup[] = [];

		for (let day = 1; day <= dayCount; day += 1) {
			const dateKey = localDateKey(new SvelteDate(year, month.monthIndex, day));
			const weekdayIndexVal = weekdayIndexForDate(dateKey);
			if (weekdayIndexVal < 0 || weekdayIndexVal > 4) continue;

			const weekKey = mondayDateKey(dateKey);
			let group = groupsByKey.get(weekKey);
			if (!group) {
				group = createMatrixWeekGroup(weekKey);
				groupsByKey.set(weekKey, group);
				groups.push(group);
			}

			group.slots[weekdayIndexVal] = enrichSlot(
				MATRIX_WEEKDAYS[weekdayIndexVal],
				datesByKey.get(dateKey) ?? null,
				dateKey,
				weekdayIndexVal === 0
			);
		}

		return groups.map((g, index) => ({
			...g,
			label: `Week ${index + 1}`
		}));
	}

	// Fallback: when no month match, build from the dates array directly
	const groupsByKey = new SvelteMap<string, MatrixWeekGroup>();
	const groups: MatrixWeekGroup[] = [];

	for (const dt of dates) {
		const weekdayIndexVal = weekdayIndexForDate(dt.date);
		if (weekdayIndexVal < 0 || weekdayIndexVal > 4) continue;

		const key = mondayDateKey(dt.date);
		let group = groupsByKey.get(key);

		if (!group) {
			group = createMatrixWeekGroup(key);
			groupsByKey.set(key, group);
			groups.push(group);
		}

		group.slots[weekdayIndexVal] = enrichSlot(
			MATRIX_WEEKDAYS[weekdayIndexVal],
			dt,
			dt.date,
			weekdayIndexVal === 0
		);
	}

	return groups.map((g, index) => ({
		...g,
		label: `Week ${index + 1}`
	}));
}

export function weekRangeLabel(group: MatrixWeekGroup) {
	const dates = group.slots.map((slot) => slot.dateKey).filter((d): d is string => d !== null);
	const first = dates[0];
	const last = dates.at(-1);
	if (!first || !last) return 'Mon-Fri';
	if (first === last) return matrixDateLabel(first);

	return `${formatWeekday(first)}-${formatWeekday(last)} / ${formatDayNumber(
		first
	)}-${formatDayNumber(last)}`;
}

export function headerReviewValue(
	draftValue: string,
	templateValue: string,
	workbookSettings: unknown
) {
	const value = workbookSettings ? draftValue : draftValue || templateValue;
	return value.trim() || 'Blank';
}

export function headerReviewMonthValue(
	draftReportMonth: string,
	templateValue: string,
	workbookSettings: unknown
) {
	const value = workbookSettings ? draftReportMonth : draftReportMonth || templateValue;
	return reportMonthLabel(value);
}
