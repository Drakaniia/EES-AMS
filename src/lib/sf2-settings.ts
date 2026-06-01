import type { Sf2WorkbookSettings } from './types';

export type Sf2SchoolMonth = {
	value: string;
	label: string;
	monthIndex: number;
};

export type Sf2CalendarCell = {
	key: string;
	day: number | null;
	label: string;
	isSchoolDay: boolean;
	isSelected: boolean;
};

export type Sf2DraftDefaults = {
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	firstSchoolDay: number;
};

const MONTH_TOKENS = [
	{ value: 'JUNE', label: 'June', monthIndex: 5, aliases: ['JUNE', 'JUN'] },
	{ value: 'JULY', label: 'July', monthIndex: 6, aliases: ['JULY', 'JUL'] },
	{ value: 'AUGUST', label: 'August', monthIndex: 7, aliases: ['AUGUST', 'AUG'] },
	{ value: 'SEPTEMBER', label: 'September', monthIndex: 8, aliases: ['SEPTEMBER', 'SEP'] },
	{ value: 'OCTOBER', label: 'October', monthIndex: 9, aliases: ['OCTOBER', 'OCT'] },
	{ value: 'NOVEMBER', label: 'November', monthIndex: 10, aliases: ['NOVEMBER', 'NOV'] },
	{ value: 'DECEMBER', label: 'December', monthIndex: 11, aliases: ['DECEMBER', 'DEC'] },
	{ value: 'JANUARY', label: 'January', monthIndex: 0, aliases: ['JANUARY', 'JAN'] },
	{ value: 'FEBRUARY', label: 'February', monthIndex: 1, aliases: ['FEBRUARY', 'FEB'] },
	{ value: 'MARCH', label: 'March', monthIndex: 2, aliases: ['MARCH', 'MAR'] },
	{ value: 'APRIL', label: 'April', monthIndex: 3, aliases: ['APRIL', 'APR'] }
] as const;

export const SF2_CALENDAR_WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

export const SF2_SCHOOL_MONTHS: Sf2SchoolMonth[] = MONTH_TOKENS.map(
	({ value, label, monthIndex }) => ({
		value,
		label,
		monthIndex
	})
);

export function normalizeSf2ReportMonth(value: string) {
	const normalized = value.trim().toUpperCase();
	if (!normalized) return '';

	const directMatch = MONTH_TOKENS.find(
		(month) => normalized === month.value || normalized === month.label.toUpperCase()
	);
	if (directMatch) return directMatch.value;

	return (
		MONTH_TOKENS.find((month) => month.aliases.some((alias) => hasMonthToken(normalized, alias)))
			?.value ?? ''
	);
}

export function sf2MonthByIndex(monthIndex: number) {
	return SF2_SCHOOL_MONTHS.find((month) => month.monthIndex === monthIndex);
}

export function sf2MonthByValue(value: string) {
	const normalized = normalizeSf2ReportMonth(value);
	return SF2_SCHOOL_MONTHS.find((month) => month.value === normalized);
}

export function defaultSf2ReportMonth(today = new Date()) {
	return sf2MonthByIndex(today.getMonth())?.value ?? 'JUNE';
}

export function defaultSf2SchoolYear(today = new Date()) {
	const currentMonthIndex = today.getMonth();
	const startYear = currentMonthIndex <= 3 ? today.getFullYear() - 1 : today.getFullYear();
	return `${startYear}-${startYear + 1}`;
}

export function sf2ReportYear(
	monthValue: string,
	schoolYear: string,
	fallbackYear = new Date().getFullYear()
) {
	const month = sf2MonthByValue(monthValue);
	const years = schoolYearYears(schoolYear);
	if (!month || !years) return fallbackYear;
	return month.monthIndex >= 5 ? years[0] : years[1];
}

export function defaultSf2FirstSchoolDay(monthValue: string, schoolYear: string) {
	const month = sf2MonthByValue(monthValue);
	if (!month) return 1;

	const year = sf2ReportYear(monthValue, schoolYear);
	const firstDayOfWeek = new Date(year, month.monthIndex, 1).getDay();
	if (firstDayOfWeek === 0) return 2;
	if (firstDayOfWeek === 6) return 3;
	return 1;
}

export function sf2MonthDayCount(monthValue: string, schoolYear: string) {
	const month = sf2MonthByValue(monthValue);
	if (!month) return 31;
	return new Date(sf2ReportYear(monthValue, schoolYear), month.monthIndex + 1, 0).getDate();
}

export function isSf2SchoolDay(monthValue: string, schoolYear: string, day: number) {
	const month = sf2MonthByValue(monthValue);
	if (!month) return false;

	const dayCount = sf2MonthDayCount(monthValue, schoolYear);
	if (day < 1 || day > dayCount) return false;

	const weekday = new Date(sf2ReportYear(monthValue, schoolYear), month.monthIndex, day).getDay();
	return weekday >= 1 && weekday <= 5;
}

export function normalizedSf2FirstSchoolDay(monthValue: string, schoolYear: string, day: number) {
	if (isSf2SchoolDay(monthValue, schoolYear, day)) return day;
	return defaultSf2FirstSchoolDay(monthValue, schoolYear);
}

export function sf2CalendarCells(
	monthValue: string,
	schoolYear: string,
	selectedDay: number
): Sf2CalendarCell[] {
	const month = sf2MonthByValue(monthValue);
	if (!month) return [];

	const year = sf2ReportYear(monthValue, schoolYear);
	const dayCount = sf2MonthDayCount(monthValue, schoolYear);
	const firstWeekday = new Date(year, month.monthIndex, 1).getDay();
	const leadingBlankCount = (firstWeekday + 6) % 7;
	const cells: Sf2CalendarCell[] = [];

	for (let index = 0; index < leadingBlankCount; index += 1) {
		cells.push({
			key: `blank-start-${index}`,
			day: null,
			label: '',
			isSchoolDay: false,
			isSelected: false
		});
	}

	for (let day = 1; day <= dayCount; day += 1) {
		const isSchoolDay = isSf2SchoolDay(monthValue, schoolYear, day);
		cells.push({
			key: `day-${day}`,
			day,
			label: String(day),
			isSchoolDay,
			isSelected: selectedDay === day
		});
	}

	while (cells.length % 7 !== 0) {
		cells.push({
			key: `blank-end-${cells.length}`,
			day: null,
			label: '',
			isSchoolDay: false,
			isSelected: false
		});
	}

	return cells;
}

export function shouldPromptForSf2SettingsUpdate(
	settings: Pick<Sf2WorkbookSettings, 'reportMonth'>,
	today = new Date()
) {
	return normalizeSf2ReportMonth(settings.reportMonth) !== defaultSf2ReportMonth(today);
}

export function sf2ImportedSettingsDraftDefaults(
	settings: Sf2WorkbookSettings,
	today = new Date()
): Sf2DraftDefaults {
	const reportMonth = defaultSf2ReportMonth(today);
	const schoolYear = schoolYearMatchesReportMonthYear(settings.schoolYear, reportMonth, today)
		? settings.schoolYear.trim()
		: defaultSf2SchoolYear(today);

	return {
		schoolId: settings.schoolId,
		schoolName: settings.schoolName,
		schoolYear,
		reportMonth,
		gradeLevel: settings.gradeLevel,
		section: settings.section,
		adviserName: settings.adviserName,
		schoolHeadName: settings.schoolHeadName,
		firstSchoolDay: defaultSf2FirstSchoolDay(reportMonth, schoolYear)
	};
}

export function sf2ReportMonthLabel(value: string) {
	return sf2MonthByValue(value)?.label ?? value.trim();
}

function hasMonthToken(value: string, token: string) {
	const escapedToken = token.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	return new RegExp(`(^|[^A-Z])${escapedToken}([^A-Z]|$)`).test(value);
}

function schoolYearYears(schoolYear: string): [number, number] | null {
	const years = schoolYear
		.split(/\D+/)
		.filter((part) => part.length === 4 && part.startsWith('20'))
		.map((part) => Number(part))
		.filter((year) => Number.isFinite(year));

	if (years.length < 2) return null;
	return [years[0], years[1]];
}

function schoolYearMatchesReportMonthYear(schoolYear: string, reportMonth: string, today: Date) {
	const month = sf2MonthByValue(reportMonth);
	const years = schoolYearYears(schoolYear);
	if (!month || !years) return false;

	const reportYear = month.monthIndex >= 5 ? years[0] : years[1];
	return reportYear === today.getFullYear();
}
