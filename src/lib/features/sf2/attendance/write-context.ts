/**
 * Resolving one month for writing — the port of
 * `sf2::month_preview::month_write_target`.
 *
 * The grid already reads a month through `month.ts`; this is the *same*
 * resolution taken in the shape a write takes. Sharing it is the point: the sheet
 * a month is written to and the sheet the grid drew cannot be different sheets,
 * and that mismatch is how "Open SF2" came to sync one month and open another.
 */

import { appError, getDriver } from '$lib/db';
import { listEvents } from '$lib/db/repos/events';
import { getSettings } from '$lib/db/repos/settings';
import type { AttendanceEvent, Student } from '$lib/domain/models';
import type { Sf2MonthTemplate } from '$lib/types';
import { sf2MonthName, sf2MonthNumber, sf2ReportYear } from '$lib/features/sf2/calendar';
import { currentYear, lastDayOfMonth } from '$lib/features/sf2/first-school-day';
import { monthWorkbookSheetName } from '$lib/features/sf2/workbook-files';
import {
	findMonthTemplate,
	latestMonthTemplateForClass,
	listMonthDateMappings
} from '$lib/features/sf2/month/templates';
import type { Sf2MonthDateMappingRecord } from '$lib/features/sf2/month/templates';
import { monthRosterForTemplate } from '$lib/features/sf2/month/students';
import type { Sf2MonthStudentMapping } from '$lib/features/sf2/month/students';
import {
	latestTemplateForClass,
	legacyDateMappingsInMonth,
	studentMappingsForTemplate,
	type Sf2TemplateRecord
} from '$lib/features/sf2/repository';
import { firstSchoolDayFromMappings } from '$lib/features/sf2/metadata';
import { absentStudentIds } from './attendance-events';
import type { Sf2OpenLayout } from './attendance-write';

/** Shown when a month is asked for that no name can resolve. */
export const UNKNOWN_MONTH_MESSAGE = 'Report month must be a valid month name';

/** The month row, roster and day grid a write to one month needs. */
export type Sf2MonthWriteContext = {
	classId: string;
	/** Canonical uppercase month name, e.g. `SEPTEMBER`. */
	reportMonth: string;
	reportYear: number;
	schoolYear: string;
	/** The worksheet this month is written to, e.g. `SEPTEMBER 2026`. */
	sheetName: string;
	/** The one workbook every month of the class lives in. */
	sourcePath: string;
	templateId: string;
	/**
	 * Set when the context fell back to a pre-split `sf2_templates` row, whose
	 * sync timestamp lives in that table rather than in `sf2_month_templates`.
	 */
	legacyTemplateId?: string;
	roster: Sf2MonthStudentMapping[];
	/** This month's day columns only, each carrying the worksheet it is on. */
	dates: Sf2MonthDateMappingRecord[];
	/** The header and calendar the open path writes before the marks. */
	layout: Sf2OpenLayout;
};

/**
 * The newest school year the class actually has month rows for, else the legacy
 * settings label, else a year derived from the clock.
 *
 * Identical in order and wording to `month.ts`'s own resolution, so the write path
 * and the read path cannot name two different years for one class.
 */
export async function resolveSchoolYear(classId: string): Promise<string> {
	const latest = await getDriver().queryOne<{ school_year: string }>(
		`SELECT school_year FROM sf2_month_templates
		 WHERE active_class_id = ?
		   AND school_year GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9]'
		 ORDER BY CAST(SUBSTR(school_year, 1, 4) AS INTEGER) DESC
		 LIMIT 1`,
		[classId]
	);
	if (latest !== undefined) return latest.school_year;

	const legacy = (await getSettings()).schoolYear?.trim() ?? '';
	if (legacy !== '') return legacy;

	const year = currentYear();
	return `${year}-${year}`;
}

/**
 * Resolve the write target for one month.
 *
 * The stored `reportYear` wins over any recomputation, for the reason `month.ts`
 * gives at length: the retired June year rule and the school-year September rule
 * disagree about AUGUST, and a month filed under the wrong year is worse than a
 * month the app refuses to open.
 *
 * Month first, latest template second: a month with no row falls back to the
 * newest `sf2_month_templates` row of the class — what the retired
 * `latest_template_for_class` would have opened — and only then to a pre-split
 * `sf2_templates` row when one is still on record. The resolved month still
 * decides all three: whose absences are written, whose day columns they land
 * in, which worksheet is activated. With no row anywhere, the refusal names the
 * missing template rather than inventing a month.
 *
 * `allowEmptyDates` is for the open path, and exists because of an ordering the
 * old Rust had: an empty date-mapping set was never a refusal there, it was the
 * guard's `NO_MAPPED_DATES` `Unmeasured`, which opens the month read-only. Rust
 * reached the missing-workbook check *inside* that read-only branch, so a month
 * with no mapped days and no file on disk answered with the missing-file
 * message. Resolving here has to hand that case back, or the refusal above
 * answers first and names the wrong thing.
 */
export async function resolveMonthWriteContext(
	classId: string,
	reportMonth: string,
	schoolYear?: string,
	allowEmptyDates = false
): Promise<Sf2MonthWriteContext> {
	const monthNumber = sf2MonthNumber(reportMonth);
	if (monthNumber === undefined) throw appError('InvalidInput', UNKNOWN_MONTH_MESSAGE);

	const month = sf2MonthName(monthNumber);
	const year = schoolYear?.trim() || (await resolveSchoolYear(classId));
	const template = await findMonthTemplate(classId, year, month);
	if (template !== undefined) return contextFromMonthRow(classId, template, allowEmptyDates);

	const latest = await latestMonthTemplateForClass(classId);
	if (latest !== undefined) return contextFromMonthRow(classId, latest, allowEmptyDates);

	const legacy = await legacyTemplateForClass(classId);
	if (legacy !== undefined) return contextFromLegacyRow(classId, legacy, allowEmptyDates);

	throw appError('InvalidInput', 'No SF2 template imported for this class');
}

/** The write target for a per-month row, whichever month the caller named. */
async function contextFromMonthRow(
	classId: string,
	template: Sf2MonthTemplate,
	allowEmptyDates: boolean
): Promise<Sf2MonthWriteContext> {
	const dates = await listMonthDateMappings(template.id);
	if (dates.length === 0 && !allowEmptyDates) {
		throw appError(
			'InvalidInput',
			`No attendance days are mapped to ${template.reportMonth} yet, so there is nothing to write.`
		);
	}

	const sheetName = monthWorkbookSheetName(template.reportMonth, template.reportYear);
	return {
		classId,
		reportMonth: template.reportMonth,
		reportYear: template.reportYear,
		schoolYear: template.schoolYear,
		sheetName,
		sourcePath: template.sourcePath,
		templateId: template.id,
		roster: await monthRosterForTemplate(template.id),
		// Every day of a month is on the same worksheet, so the month being resolved
		// is the authority over any stored per-day name - the same call the grid read
		// makes, and for the same reason.
		dates: dates.map((date) => ({ ...date, sheetName })),
		layout: {
			metadata: {
				schoolId: template.schoolId ?? '',
				schoolName: template.schoolName ?? '',
				schoolYear: template.schoolYear,
				reportMonth: template.reportMonth,
				gradeLevel: template.gradeLevel ?? '',
				section: template.section ?? '',
				adviserName: template.adviserName ?? '',
				schoolHeadName: template.schoolHeadName ?? '',
				firstSchoolDay: template.firstSchoolDayOverride ?? template.firstSchoolDay ?? 1
			},
			firstSchoolDay: template.firstSchoolDayOverride ?? template.firstSchoolDay ?? 1
		}
	};
}

/**
 * The write target for a pre-split `sf2_templates` row, scoped to that row's
 * own report month.
 *
 * The pre-split date table is keyed by full date rather than by month, so the
 * month's day columns are read as the closed date range the row's month names.
 * Only consulted when no per-month row exists at all; the tables may themselves
 * be absent (a database created by this build never had them).
 */
async function contextFromLegacyRow(
	classId: string,
	legacy: Sf2TemplateRecord,
	allowEmptyDates: boolean
): Promise<Sf2MonthWriteContext> {
	const monthNumber = sf2MonthNumber(legacy.reportMonth);
	if (monthNumber === undefined) throw appError('InvalidInput', UNKNOWN_MONTH_MESSAGE);
	const month = sf2MonthName(monthNumber);
	const reportYear = sf2ReportYear(legacy.schoolYear, monthNumber);
	const start = `${reportYear}-${String(monthNumber).padStart(2, '0')}-01`;
	const end = `${reportYear}-${String(monthNumber).padStart(2, '0')}-${String(lastDayOfMonth(reportYear, monthNumber)).padStart(2, '0')}`;

	const stored = await legacyDateMappingsInMonth(legacy.id, start, end);
	if (stored.length === 0 && !allowEmptyDates) {
		throw appError(
			'InvalidInput',
			`No attendance days are mapped to ${month} yet, so there is nothing to write.`
		);
	}

	const sheetName = monthWorkbookSheetName(month, reportYear);
	const dates = stored.map((mapping) => ({
		templateId: mapping.templateId,
		date: mapping.date,
		columnLetter: mapping.columnLetter,
		columnIndex: mapping.columnIndex,
		sheetName: mapping.sheetName === '' ? sheetName : mapping.sheetName
	}));
	return {
		classId,
		reportMonth: month,
		reportYear,
		schoolYear: legacy.schoolYear,
		sheetName,
		sourcePath: legacy.sourcePath,
		templateId: legacy.id,
		legacyTemplateId: legacy.id,
		roster: (await studentMappingsForTemplate(legacy.id)).map((mapping) => ({
			templateId: mapping.templateId,
			studentId: mapping.studentId,
			workbookName: mapping.workbookName,
			normalizedName: mapping.normalizedName,
			rowIndex: mapping.rowIndex,
			genderBlock: mapping.genderBlock
		})),
		dates,
		layout: {
			metadata: {
				schoolId: legacy.schoolId,
				schoolName: legacy.schoolName,
				schoolYear: legacy.schoolYear,
				reportMonth: month,
				gradeLevel: legacy.gradeLevel,
				section: legacy.section,
				adviserName: legacy.adviserName,
				schoolHeadName: legacy.schoolHeadName,
				firstSchoolDay: firstSchoolDayFromMappings(dates)
			},
			firstSchoolDay: firstSchoolDayFromMappings(dates)
		}
	};
}

/** A pre-split template row, or `undefined` when the tables are gone. */
async function legacyTemplateForClass(classId: string): Promise<Sf2TemplateRecord | undefined> {
	const tables = await getDriver().query<{ name: string }>(
		`SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('sf2_templates', 'sf2_student_mappings', 'sf2_date_mappings')`
	);
	if (tables.length < 3) return undefined;
	return latestTemplateForClass(classId);
}

/**
 * The learners with an explicit absence on each of the month's mapped days.
 *
 * Built once for the whole month rather than per day: the same event list is
 * scanned for every day, and a per-day query would re-read the month `n` times.
 *
 * Keys are insertion-ordered by the dates list, so a caller iterating the map gets
 * the days in the order the workbook addresses them.
 */
export function absentIdsByDate(
	dates: readonly Sf2MonthDateMappingRecord[],
	events: readonly AttendanceEvent[],
	students: readonly Student[],
	classId: string
): Map<string, Set<string>> {
	return new Map(
		dates.map((date) => [date.date, absentStudentIds(events, students, classId, date.date)])
	);
}

/**
 * The absence lookup a write needs, read once.
 *
 * `events` and `students` are passed in rather than fetched here so the caller
 * controls the read and a test can drive the workbook arithmetic without a
 * database.
 */
export async function monthAbsences(
	context: Sf2MonthWriteContext,
	students: readonly Student[]
): Promise<Map<string, Set<string>>> {
	return absentIdsByDate(context.dates, await listEvents(), students, context.classId);
}
