import { SvelteDate, SvelteMap } from 'svelte/reactivity';
import type { Sf2PreviewDate } from '$lib/types';
import { sf2MonthByValue, sf2ReportMonthLabel } from '$lib/features/settings/sf2-workbook';
import type { Sf2PreviewCell, Sf2PreviewStudentRow } from '$lib/db-rust';

// ── Types ──────────────────────────────────────────────────────────────────────

export const MATRIX_WEEKDAYS = ['M', 'T', 'W', 'TH', 'F'] as const;

export type MatrixWeekday = (typeof MATRIX_WEEKDAYS)[number];

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

export type MatrixWeekGroupHeader = {
	key: string;
	label: string;
	slots: MatrixDateSlot[];
	rangeLabel: string;
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
	return fallback;
}

export function formatDate(date: string) {
	const value = new SvelteDate(`${date}T00:00:00`);
	return value.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

export function formatWeekday(date: string) {
	const value = new SvelteDate(`${date}T00:00:00`);
	return value.toLocaleDateString(undefined, { weekday: 'short' });
}

export function formatDayNumber(date: string) {
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

export function cellKey(studentId: string, date: string) {
	return `${studentId}:${date}`;
}

export function cellLabel(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
	const state = cell.status === 'absent' ? 'absent' : 'present';
	return `${row.studentName}, ${matrixDateLabel(cell.date)}: ${state}`;
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

		const cellsByDate = new SvelteMap<string, Sf2PreviewCell>();
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

export function cellClass(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
	if (!row.mapped) return 'border-border bg-surface text-muted-foreground';
	if (cell.status === 'absent') return 'border-red-500/35 bg-red-50 text-red-700';
	// Present/Open = visually empty (no green background, no checkmark)
	return 'border-border bg-background text-muted-foreground';
}

export function reportMonthLabel(value: string) {
	return sf2ReportMonthLabel(value) || 'Blank';
}

export function createMatrixWeekGroup(key: string): MatrixWeekGroup {
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

export function mondayDateKey(date: string) {
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

export function localDateKey(date: Date) {
	const year = date.getFullYear();
	const month = String(date.getMonth() + 1).padStart(2, '0');
	const day = String(date.getDate()).padStart(2, '0');
	return `${year}-${month}-${day}`;
}

export function buildMatrixWeekGroups(
	dates: Sf2PreviewDate[],
	reportMonth: string
): MatrixWeekGroup[] {
	const month = sf2MonthByValue(reportMonth);

	// Pre-index dates by dateKey for O(1) lookup instead of O(n) Array.find per slot
	const datesByKey = new SvelteMap<string, Sf2PreviewDate>();
	for (const d of dates) {
		datesByKey.set(d.date, d);
	}

	if (month) {
		const year =
			dates.length > 0 ? Number(dates[0].date.split('-')[0]) : new SvelteDate().getFullYear();
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

/**
 * Generate skeleton preview dates for a month — all weekdays with empty SF2
 * mappings. This lets the calendar grid render instantly after a month switch
 * before the full preview loads from the backend.
 */
export function buildSkeletonDates(reportMonth: string, schoolYear?: string): Sf2PreviewDate[] {
	const month = sf2MonthByValue(reportMonth);
	if (!month) return [];

	// Derive the calendar year from the school year if available
	let year = new SvelteDate().getFullYear();
	if (schoolYear) {
		const startYear = parseInt(schoolYear.split('-')[0], 10);
		if (!isNaN(startYear)) {
			// If report month is June–December, use the start year;
			// if January–May, use startYear + 1 (next calendar year)
			year = month.monthIndex >= 5 ? startYear : startYear + 1;
		}
	}

	const dayCount = new SvelteDate(year, month.monthIndex + 1, 0).getDate();
	const dates: Sf2PreviewDate[] = [];

	for (let day = 1; day <= dayCount; day += 1) {
		const dateKey = localDateKey(new SvelteDate(year, month.monthIndex, day));
		const weekdayIndexVal = weekdayIndexForDate(dateKey);
		// Skip weekends
		if (weekdayIndexVal < 0 || weekdayIndexVal > 4) continue;

		dates.push({
			date: dateKey,
			sheetName: '',
			columnLetter: '',
			columnIndex: 0
		});
	}

	return dates;
}

export function headerReviewMonthValue(
	draftReportMonth: string,
	templateValue: string,
	workbookSettings: unknown
) {
	const value = workbookSettings ? draftReportMonth : draftReportMonth || templateValue;
	return reportMonthLabel(value);
}
