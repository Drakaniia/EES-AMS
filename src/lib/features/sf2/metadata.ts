/**
 * SF2 metadata: the eight header fields, and the date mappings that go with them.
 *
 * The rule this file exists to hold together is that a metadata block and the date
 * grid it describes are one thing:
 * `metadataFromImportAnalysis` derives the first attendance day *from* the dates
 * the workbook recorded, so the header block cannot claim a first day the grid
 * does not have.
 */

import { invalidInput } from '$lib/db';
import type { Sf2TemplateDraft } from '$lib/types';
import {
	firstSchoolDayForReportMonth,
	sf2MonthNumber,
	sf2ReportYear,
	validateFirstSchoolDay
} from './calendar';
import type { Sf2TemplateMetadata, Sf2WorkbookAnalysis } from './calendar';
import { naiveDate, parseIsoDate } from './first-school-day';

export type { Sf2TemplateMetadata, Sf2WorkbookAnalysis } from './calendar';

/** One row of `sf2_date_mappings`: a day column and the date it stands for. */
export type Sf2DateMapping = {
	templateId: string;
	sheetName: string;
	/** `YYYY-MM-DD`. */
	date: string;
	columnLetter: string;
	/** 1-based, matching Excel. */
	columnIndex: number;
};

/** The template columns the date rules need: which month, which year. */
type ReportPeriod = { reportMonth: string; schoolYear: string };

/**
 * Whether the stored date mappings still describe this template's report month.
 *
 * "Still" is the whole question: a teacher who changes the report month on the
 * settings screen has made every stored mapping stale, and the app has to rebuild
 * the grid rather than write marks into the previous month's columns.
 */
export function dateMappingsAreCurrentForReportMonth(
	template: ReportPeriod,
	dateMappings: readonly Sf2DateMapping[]
): boolean {
	const month = sf2MonthNumber(template.reportMonth);
	if (month === undefined) return false;

	const year = sf2ReportYear(template.schoolYear, month);
	const yearText = String(year);

	return (
		dateMappings.length > 0 &&
		dateMappings.some((mapping) => {
			const date = parseIsoDate(mapping.date);
			if (date === undefined) return false;
			const weekday = date.getUTCDay();
			return (
				date.getUTCFullYear() === year &&
				date.getUTCMonth() + 1 === month &&
				weekday >= 1 &&
				weekday <= 5 &&
				sf2MonthNumber(mapping.sheetName) === month &&
				mapping.sheetName.includes(yearText)
			);
		})
	);
}

/** The `sf2_date_mappings` rows an analysis implies. */
export function dateMappingsFromAnalysis(
	templateId: string,
	analysis: Sf2WorkbookAnalysis
): Sf2DateMapping[] {
	return analysis.dates.map((date) => ({
		templateId,
		sheetName: date.sheetName,
		date: date.date,
		columnLetter: date.columnLetter,
		columnIndex: date.columnIndex
	}));
}

/**
 * The eight header fields, trimmed.
 *
 * `configureCalendar` is false and there is no first day: this is the header on its
 * own, for a template row that is only being described, never written from.
 */
export function metadataFromAnalysis(analysis: Sf2WorkbookAnalysis): Sf2TemplateMetadata {
	return {
		schoolId: analysis.schoolId.trim(),
		schoolName: analysis.schoolName.trim(),
		schoolYear: analysis.schoolYear.trim(),
		reportMonth: analysis.reportMonth.trim(),
		gradeLevel: analysis.gradeLevel.trim(),
		section: analysis.section.trim(),
		adviserName: analysis.adviserName.trim(),
		schoolHeadName: analysis.schoolHeadName.trim(),
		configureCalendar: false
	};
}

/**
 * The header fields of an imported workbook, plus the calendar it implies.
 *
 * The calendar *is* configured on import — the teacher chose a report month by
 * handing the app a workbook for it — and the first day is derived from the dates
 * that workbook already recorded rather than from the first of the month, so a
 * class that starts late keeps the late start the teacher drew.
 */
export function metadataFromImportAnalysis(analysis: Sf2WorkbookAnalysis): Sf2TemplateMetadata {
	const metadata = metadataFromAnalysis(analysis);
	if (sf2MonthNumber(metadata.reportMonth) === undefined) return metadata;

	return {
		...metadata,
		configureCalendar: true,
		firstSchoolDay: firstSchoolDayForReportMonth(
			metadata.reportMonth,
			metadata.schoolYear,
			analysis.dates.map((date) => date.date)
		)
	};
}

function requiredDraftText(value: string, label: string): string {
	const trimmed = value.trim();
	if (trimmed === '') throw invalidInput(`${label} is required`);
	return trimmed;
}

function requiredFirstSchoolDay(
	day: number | undefined,
	reportMonth: string,
	schoolYear: string
): number {
	if (day === undefined) {
		throw invalidInput('First attendance day is required for SF2 templates');
	}
	validateFirstSchoolDay(day, reportMonth, schoolYear);
	return day;
}

/**
 * The header fields a teacher typed, validated.
 *
 * Every field is required here, unlike an imported workbook where a blank cell is
 * just a blank cell: a template written from a draft is one the app will write
 * attendance into, and a blank school year in it is a workbook nobody can find
 * again.
 */
export function metadataFromDraft(draft: Sf2TemplateDraft): Sf2TemplateMetadata {
	const schoolYear = requiredDraftText(draft.schoolYear, 'School Year');
	const reportMonth = requiredDraftText(draft.reportMonth, 'Report Month');
	const firstSchoolDay = requiredFirstSchoolDay(draft.firstSchoolDay, reportMonth, schoolYear);

	return {
		schoolId: requiredDraftText(draft.schoolId, 'School ID'),
		schoolName: requiredDraftText(draft.schoolName, 'Name of School'),
		schoolYear,
		reportMonth,
		gradeLevel: requiredDraftText(draft.gradeLevel, 'Grade Level'),
		section: requiredDraftText(draft.section, 'Section'),
		adviserName: requiredDraftText(draft.adviserName, 'Adviser / LIS Name'),
		schoolHeadName: requiredDraftText(draft.schoolHeadName, 'School Head Name'),
		configureCalendar: true,
		firstSchoolDay
	};
}

/** The header fields of a stored template row, for re-describing it. */
export function templateMetadata(template: Sf2TemplateMetadata): Sf2TemplateMetadata {
	return { ...template, configureCalendar: false, firstSchoolDay: undefined };
}

/**
 * Every header field the workbook leaves blank, named the way the form names it.
 *
 * These are what a teacher has to type in by hand before the submission is
 * complete, so each line says where on the form the field belongs rather than just
 * naming the column it came from.
 */
export function sf2MetadataWarnings(metadata: Sf2TemplateMetadata): string[] {
	const fields: [string, string][] = [
		['School ID', metadata.schoolId],
		['Name of School', metadata.schoolName],
		['School Year', metadata.schoolYear],
		['Report for the Month of', metadata.reportMonth],
		['Grade Level', metadata.gradeLevel],
		['Section', metadata.section],
		[
			'Signature of Adviser over Printed Name / Generated thru LIS adviser name',
			metadata.adviserName
		],
		['Signature of School Head over Printed Name', metadata.schoolHeadName]
	];

	return fields
		.filter(([, value]) => value.trim() === '')
		.map(([label]) => `${label} is blank in this SF2 workbook.`);
}

/**
 * The stored mappings of the report month, re-dated into the report year.
 *
 * A mapping recorded under a different school year keeps its month and day but
 * takes the year the template now claims, so changing the school year re-dates the
 * grid instead of emptying it.
 */
export function sf2DateMappingsForReportMonth(
	template: ReportPeriod,
	dateMappings: readonly Sf2DateMapping[]
): Sf2DateMapping[] {
	const month = sf2MonthNumber(template.reportMonth);
	if (month === undefined) return [];

	const year = sf2ReportYear(template.schoolYear, month);
	const mappings: Sf2DateMapping[] = [];
	for (const mapping of dateMappings) {
		const date = parseIsoDate(mapping.date);
		if (date === undefined || date.getUTCMonth() + 1 !== month) continue;
		const normalized = naiveDate(year, month, date.getUTCDate());
		if (normalized === undefined) continue;

		mappings.push({
			...mapping,
			date: `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}-${String(date.getUTCDate()).padStart(2, '0')}`
		});
	}
	return mappings;
}

/**
 * The earliest day the mappings recorded, or 1 when they record none.
 *
 * The fallback is the grid's own anchor rather than a claim about the class: it is
 * what "start from the beginning of the month" means to the date-header writer, and
 * it says nothing about when classes actually started.
 */
export function firstSchoolDayFromMappings(dateMappings: readonly Sf2DateMapping[]): number {
	let earliest: number | undefined;
	for (const mapping of dateMappings) {
		const date = parseIsoDate(mapping.date);
		if (date === undefined) continue;
		const day = date.getUTCDate();
		if (earliest === undefined || day < earliest) earliest = day;
	}
	return earliest ?? 1;
}
