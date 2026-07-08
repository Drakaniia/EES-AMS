import { SvelteDate } from 'svelte/reactivity';
import type { Sf2PreviewDate } from '$lib/types';
import { sf2MonthByValue, sf2ReportMonthLabel } from '$lib/features/settings/sf2-workbook';
import type { Sf2PreviewCell, Sf2PreviewStudentRow } from '$lib/db-rust';

// ── Types ──────────────────────────────────────────────────────────────────────

export const MATRIX_WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri'] as const;

export type MatrixWeekday = (typeof MATRIX_WEEKDAYS)[number];

export type MatrixDateSlot = {
	key: string;
	weekday: MatrixWeekday;
	date: Sf2PreviewDate | null;
	dateKey: string | null;
};

export type MatrixWeekGroup = {
	key: string;
	label: string;
	slots: MatrixDateSlot[];
};

export type MatrixStudentRow = Sf2PreviewStudentRow & {
	cellsByDate: Map<string, Sf2PreviewCell>;
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
	const state = cell.status === 'present' ? 'present' : 'absent';
	return `${row.studentName}, ${matrixDateLabel(cell.date)}: ${state}`;
}

export function cellForDate(row: MatrixStudentRow, date: string) {
	return row.cellsByDate.get(date);
}

export function unmappedCellLabel(row: Sf2PreviewStudentRow, date: string) {
	return `${row.studentName}, ${matrixDateLabel(date)}: no SF2 column mapped`;
}

export function cellClass(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
	if (!row.mapped) return 'border-border bg-surface text-muted-foreground';
	if (cell.status === 'present') return 'border-emerald-500/30 bg-emerald-50 text-emerald-700';
	if (cell.status === 'absent') return 'border-red-500/35 bg-red-50 text-red-700';
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
			dateKey: null
		}))
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
	const groups: MatrixWeekGroup[] = [];
	const month = sf2MonthByValue(reportMonth);

	if (month) {
		const year = new SvelteDate().getFullYear();
		const dayCount = new SvelteDate(year, month.monthIndex + 1, 0).getDate();

		for (let day = 1; day <= dayCount; day += 1) {
			const dateKey = localDateKey(new SvelteDate(year, month.monthIndex, day));
			const weekdayIndexVal = weekdayIndexForDate(dateKey);
			if (weekdayIndexVal < 0 || weekdayIndexVal > 4) continue;

			let group = groups.find((item) => item.key === mondayDateKey(dateKey));
			if (!group) {
				group = createMatrixWeekGroup(mondayDateKey(dateKey));
				groups.push(group);
			}

			group.slots[weekdayIndexVal] = {
				key: dateKey,
				weekday: MATRIX_WEEKDAYS[weekdayIndexVal],
				date: dates.find((d) => d.date === dateKey) ?? null,
				dateKey
			};
		}

		return groups.map((g, index) => ({
			...g,
			label: `Week ${index + 1}`
		}));
	}

	for (const dt of dates) {
		const weekdayIndexVal = weekdayIndexForDate(dt.date);
		if (weekdayIndexVal < 0 || weekdayIndexVal > 4) continue;

		const key = mondayDateKey(dt.date);
		let group = groups.find((item) => item.key === key);

		if (!group) {
			group = createMatrixWeekGroup(key);
			groups.push(group);
		}

		group.slots[weekdayIndexVal] = {
			key: dt.date,
			weekday: MATRIX_WEEKDAYS[weekdayIndexVal],
			date: dt,
			dateKey: dt.date
		};
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
