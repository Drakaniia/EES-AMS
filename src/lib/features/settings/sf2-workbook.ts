import {
	isSchoolDay,
	lastDayOfMonth,
	naiveDate,
	schoolYearYears
} from '$lib/features/sf2/first-school-day';

type Sf2SchoolMonth = {
	value: string;
	label: string;
	monthIndex: number;
};

type Sf2DraftDefaults = {
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

type Sf2WorkbookDraftFields = Sf2DraftDefaults & {
	classId?: string;
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

export const SF2_SCHOOL_MONTHS: Sf2SchoolMonth[] = MONTH_TOKENS.map(
	({ value, label, monthIndex }) => ({
		value,
		label,
		monthIndex
	})
);

function normalizeSf2ReportMonth(value: string) {
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

function sf2MonthByIndex(monthIndex: number) {
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

/**
 * The calendar year a report month falls in, wrapping the school year at June.
 *
 * Deliberately *not* `reportYearForSchoolMonth`, which wraps at September: the two
 * rules disagree for June, July and August, and the legacy SF2 sheet names this
 * screen writes were made with the June wrap. Only the year parsing is shared.
 */
function sf2ReportYear(
	monthValue: string,
	schoolYear: string,
	fallbackYear = new Date().getFullYear()
) {
	const month = sf2MonthByValue(monthValue);
	if (!month) return fallbackYear;

	const years = schoolYearYears(schoolYear);
	if (years.length < 2) return fallbackYear;

	// School year spans June-May. Month indices: June=5...December=11, January=0...May=4.
	// Months >= 5 (June onwards) use the start year; months < 5 use the end year.
	return month.monthIndex >= 5 ? years[0] : years[1];
}

function defaultSf2FirstSchoolDay(monthValue: string, schoolYear: string) {
	const month = sf2MonthByValue(monthValue);
	if (!month) return 1;

	const monthIndex = month.monthIndex + 1;
	const reportYear = sf2ReportYear(monthValue, schoolYear);
	for (let day = 1; day <= lastDayOfMonth(reportYear, monthIndex); day += 1) {
		const date = naiveDate(reportYear, monthIndex, day);
		if (date !== undefined && isSchoolDay(date)) return day;
	}
	return 1;
}

function sf2MonthDayCount(monthValue: string, schoolYear: string) {
	const month = sf2MonthByValue(monthValue);
	if (!month) return 31;
	return lastDayOfMonth(sf2ReportYear(monthValue, schoolYear), month.monthIndex + 1);
}

function isSf2SchoolDay(monthValue: string, schoolYear: string, day: number) {
	const month = sf2MonthByValue(monthValue);
	if (!month) return false;

	if (day < 1 || day > sf2MonthDayCount(monthValue, schoolYear)) return false;

	const date = naiveDate(sf2ReportYear(monthValue, schoolYear), month.monthIndex + 1, day);
	return date !== undefined && isSchoolDay(date);
}

export function normalizedSf2FirstSchoolDay(monthValue: string, schoolYear: string, day: number) {
	if (isSf2SchoolDay(monthValue, schoolYear, day)) return day;
	return defaultSf2FirstSchoolDay(monthValue, schoolYear);
}

export function sf2ReportMonthLabel(value: string) {
	return sf2MonthByValue(value)?.label ?? value.trim();
}

export function newSf2WorkbookDraftFields(today = new Date()): Sf2WorkbookDraftFields {
	const reportMonth = defaultSf2ReportMonth(today);
	const schoolYear = defaultSf2SchoolYear(today);
	return {
		schoolId: '',
		schoolName: '',
		schoolYear,
		reportMonth,
		gradeLevel: '',
		section: '',
		adviserName: '',
		schoolHeadName: '',
		firstSchoolDay: defaultSf2FirstSchoolDay(reportMonth, schoolYear)
	};
}

function hasMonthToken(value: string, token: string) {
	const escapedToken = token.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	return new RegExp(`(^|[^A-Z])${escapedToken}([^A-Z]|$)`).test(value);
}
