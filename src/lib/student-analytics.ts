import type { AttendanceEvent, Settings } from './types';

type QuarterLabel = '1st Quarter' | '2nd Quarter' | '3rd Quarter';
type QuarterDateKey = 'q1Start' | 'q1End' | 'q2Start' | 'q2End' | 'q3Start' | 'q3End';

interface QuarterDefinition {
	startKey: QuarterDateKey;
	endKey: QuarterDateKey;
	fallbackStartMonth: number;
	fallbackEndMonth: number;
}

export interface QuarterDateRange {
	startDate: string;
	endDate: string;
	source: 'settings' | 'calendar';
}

const quarterDefinitions: Record<QuarterLabel, QuarterDefinition> = {
	'1st Quarter': {
		startKey: 'q1Start',
		endKey: 'q1End',
		fallbackStartMonth: 0,
		fallbackEndMonth: 2
	},
	'2nd Quarter': {
		startKey: 'q2Start',
		endKey: 'q2End',
		fallbackStartMonth: 3,
		fallbackEndMonth: 5
	},
	'3rd Quarter': {
		startKey: 'q3Start',
		endKey: 'q3End',
		fallbackStartMonth: 6,
		fallbackEndMonth: 8
	}
};

function isQuarterLabel(value: string | undefined): value is QuarterLabel {
	return value === '1st Quarter' || value === '2nd Quarter' || value === '3rd Quarter';
}

function normalizeIsoDate(value: string | undefined): string | null {
	const date = value?.trim().slice(0, 10);
	return date && /^\d{4}-\d{2}-\d{2}$/.test(date) ? date : null;
}

function formatCalendarDate(year: number, month: number, day: number): string {
	return `${year}-${String(month + 1).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

export function getQuarterDateRange(
	settings: Pick<
		Settings,
		'quarter' | 'q1Start' | 'q1End' | 'q2Start' | 'q2End' | 'q3Start' | 'q3End'
	> | null,
	fallbackYear: number
): QuarterDateRange | null {
	if (!settings || !isQuarterLabel(settings.quarter)) return null;

	const definition = quarterDefinitions[settings.quarter];
	const configuredStart = normalizeIsoDate(settings[definition.startKey]);
	const configuredEnd = normalizeIsoDate(settings[definition.endKey]);

	if (configuredStart && configuredEnd && configuredStart <= configuredEnd) {
		return {
			startDate: configuredStart,
			endDate: configuredEnd,
			source: 'settings'
		};
	}

	const lastFallbackDay = new Date(fallbackYear, definition.fallbackEndMonth + 1, 0).getDate();

	return {
		startDate: formatCalendarDate(fallbackYear, definition.fallbackStartMonth, 1),
		endDate: formatCalendarDate(fallbackYear, definition.fallbackEndMonth, lastFallbackDay),
		source: 'calendar'
	};
}

export function getAttendanceEventDate(event: Pick<AttendanceEvent, 'timestamp'>): string | null {
	return normalizeIsoDate(event.timestamp);
}

export function isAttendanceEventInQuarter(
	event: Pick<AttendanceEvent, 'timestamp'>,
	settings: Pick<
		Settings,
		'quarter' | 'q1Start' | 'q1End' | 'q2Start' | 'q2End' | 'q3Start' | 'q3End'
	> | null,
	fallbackYear: number
): boolean {
	const eventDate = getAttendanceEventDate(event);
	const range = getQuarterDateRange(settings, fallbackYear);

	return Boolean(eventDate && range && eventDate >= range.startDate && eventDate <= range.endDate);
}

export function filterAttendanceEventsForQuarter<T extends Pick<AttendanceEvent, 'timestamp'>>(
	events: T[],
	settings: Pick<
		Settings,
		'quarter' | 'q1Start' | 'q1End' | 'q2Start' | 'q2End' | 'q3Start' | 'q3End'
	> | null,
	fallbackYear: number
): T[] {
	return events.filter((event) => isAttendanceEventInQuarter(event, settings, fallbackYear));
}
