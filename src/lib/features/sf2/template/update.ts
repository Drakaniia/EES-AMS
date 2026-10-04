import { invalidInput } from '$lib/db';
import type { Workbook } from 'exceljs';
import { getClass } from '$lib/db/repos/classes';
import { nowEpochSeconds } from '$lib/domain/models';
import {
	SF2_FRESH_FEMALE_SLOTS,
	SF2_FRESH_FEMALE_START_ROW,
	SF2_FRESH_FEMALE_TOTAL_ROW,
	SF2_FRESH_MALE_SLOTS,
	SF2_FRESH_MALE_TOTAL_ROW,
	SF2_FIRST_LEARNER_ROW
} from '$lib/features/excel/constants';
import {
	clearTotalRows,
	writeFormulaMarks,
	writeMarks,
	writeMarksForce,
	writeMetadata
} from '$lib/features/excel/marks';
import { expandRosterRows, hideEmptyLearnerRows } from '$lib/features/excel/roster';
import { openWorkbook, saveWorkbookAtomic, sf2MonthlySheets } from '$lib/features/excel/workbook';
import { getFileSystem } from '$lib/platform/fs';
import { configureSf2Calendar, validateConfiguredCalendar } from '../calendar';
import type { Sf2TemplateMetadata } from '../calendar';
import { dateMappingsFromAnalysis, metadataFromDraft } from '../metadata';
import { layoutFingerprint } from '../workbook-files';
import { refreshMonthIdentityFields } from '../month/templates';
import {
	latestTemplateForClass,
	studentMappingsForTemplate,
	updateTemplateWithMappings
} from '../repository';
import { readWorkbookAnalysis } from '../roster/analysis';
import { clearUnusedLearnerMarks, genderCounts } from '../roster/helpers';
import { mappedSheetNames, rosterSyncFormulaMarks } from '../roster/formula-marks';
import { rosterStudentsForDraft, syncWorkbookLearnerMappings } from '../roster/learner-sync';
import {
	bundledTemplateTotalRows,
	rosterNameMarks,
	studentMappingsFromRosterAssignments,
	templateOwnsRoster,
	templateRosterAssignments
} from '../roster/parser';
import type { Sf2ImportSummary, Sf2TemplateDraft } from '$lib/types';
import type { Sf2StudentMappingRecord, Sf2TemplateRecord } from '../repository';

/**
 * Rewrite a class's existing workbook from a fresh draft.
 *
 * Two shapes, decided by {@link templateOwnsRoster} rather than by the caller:
 * a bundled working copy is re-laid-out from scratch, while a workbook the school
 * handed over only learns about the learners it does not have yet.
 */

/** The header block, as the metadata writer takes it. */
function headerBlock(metadata: Sf2TemplateMetadata) {
	return {
		schoolId: metadata.schoolId,
		schoolName: metadata.schoolName,
		schoolYear: metadata.schoolYear,
		reportMonth: metadata.reportMonth,
		gradeLevel: metadata.gradeLevel,
		section: metadata.section,
		adviserName: metadata.adviserName,
		schoolHeadName: metadata.schoolHeadName,
		firstSchoolDay: metadata.firstSchoolDay ?? 1
	};
}

/** What the workbook work produced, whatever branch it came from. */
interface WorkbookOutcome {
	analysis: ReturnType<typeof readWorkbookAnalysis>;
	studentMappings: Sf2StudentMappingRecord[];
	studentsCreated: number;
	studentsReused: number;
	learnersFound: number;
}

/** Update an existing workbook's settings, optionally recreating the roster. */
export async function updateWorkbookSettings(draft: Sf2TemplateDraft): Promise<Sf2ImportSummary> {
	const metadata = metadataFromDraft(draft);
	const classId = draft.classId?.trim();
	if (classId === undefined || classId === '') throw invalidInput('Class is required');

	const existing = await latestTemplateForClass(classId);
	if (existing === undefined) throw invalidInput('No SF2 workbook imported for this class');
	if (!(await getFileSystem().exists(existing.sourcePath))) {
		throw invalidInput(
			'The app SF2 working workbook no longer exists. Import the SF2 workbook again'
		);
	}

	const classRecord = await getClass(classId);
	if (classRecord === undefined) throw invalidInput('Selected class was not found');

	const workbook = await openWorkbook(existing.sourcePath);
	writeMetadata(workbook, headerBlock(metadata));
	// The header and the calendar are written together; they are one thing, because
	// a header naming a report month whose day row still holds another month's dates
	// is a workbook that will not validate.
	configureSf2Calendar(workbook, metadata);
	const outcome = templateOwnsRoster(existing)
		? await rewriteBundledRoster(workbook, existing, classId, draft)
		: await adoptWorkbookRoster(workbook, existing, classId);

	await saveWorkbookAtomic(workbook, existing.sourcePath);
	validateConfiguredCalendar(outcome.analysis, metadata);

	const dateMappings = dateMappingsFromAnalysis(existing.id, outcome.analysis);
	const template: Sf2TemplateRecord = {
		...existing,
		schoolId: metadata.schoolId,
		schoolName: metadata.schoolName,
		schoolYear: metadata.schoolYear,
		reportMonth: metadata.reportMonth,
		gradeLevel: metadata.gradeLevel,
		section: metadata.section,
		adviserName: metadata.adviserName,
		schoolHeadName: metadata.schoolHeadName,
		layoutFingerprint: layoutFingerprint(outcome.analysis),
		activeClassId: classRecord.id,
		importedAt: nowEpochSeconds(),
		lastSyncedAt: undefined
	};
	await updateTemplateWithMappings(template, outcome.studentMappings, dateMappings);
	// The month rows carry their own copies of the class-level header, and opening
	// a month stamps the sheet from the month row — so without this the next open
	// stamps the edited names straight back to the stale ones.
	await refreshMonthIdentityFields(classId, metadata.schoolYear, {
		schoolId: metadata.schoolId,
		schoolName: metadata.schoolName,
		gradeLevel: metadata.gradeLevel,
		section: metadata.section,
		adviserName: metadata.adviserName,
		schoolHeadName: metadata.schoolHeadName
	});

	return {
		templateId: template.id,
		classId: classRecord.id,
		className: classRecord.name,
		sourcePath: template.sourcePath,
		schoolYear: template.schoolYear,
		gradeLevel: template.gradeLevel,
		section: template.section,
		learnersFound: outcome.learnersFound,
		studentsCreated: outcome.studentsCreated,
		studentsReused: outcome.studentsReused,
		datesMapped: dateMappings.length
	};
}

/**
 * A bundled working copy: every student is re-assigned to a slot and the workbook
 * grows if the class has outgrown it.
 */
async function rewriteBundledRoster(
	workbook: Workbook,
	template: Sf2TemplateRecord,
	classId: string,
	draft: Sf2TemplateDraft
): Promise<WorkbookOutcome> {
	const { students, created, reused } = await rosterStudentsForDraft(classId, draft.learnerNames);
	const assignments = templateRosterAssignments(students);
	const { maleCount, femaleCount } = genderCounts(students);
	const rows = bundledTemplateTotalRows(maleCount, femaleCount);

	// The workbook's own capacity, from the mappings already on record - not from the
	// class, which may have shrunk since the last time it grew.
	const existingMappings = await studentMappingsForTemplate(template.id);
	const existingMaleMapped = countBlock(existingMappings, 'MALE');
	const existingFemaleMapped = countBlock(existingMappings, 'FEMALE');
	const currentMaleCapacity = Math.max(existingMaleMapped, SF2_FRESH_MALE_SLOTS);
	const currentFemaleCapacity = Math.max(existingFemaleMapped, SF2_FRESH_FEMALE_SLOTS);
	const extraMale = Math.max(0, maleCount - currentMaleCapacity);
	const extraFemale = Math.max(0, femaleCount - currentFemaleCapacity);

	if (extraMale > 0 || extraFemale > 0) {
		const isFresh = existingMappings.length === 0;
		expandRosterRows(
			workbook,
			extraMale,
			extraFemale,
			isFresh ? SF2_FRESH_MALE_TOTAL_ROW : SF2_FIRST_LEARNER_ROW + currentMaleCapacity,
			isFresh
				? SF2_FRESH_FEMALE_TOTAL_ROW
				: SF2_FRESH_FEMALE_START_ROW +
						(currentMaleCapacity - SF2_FRESH_MALE_SLOTS) +
						currentFemaleCapacity
		);
	}

	const analysis = readWorkbookAnalysis(workbook);
	const sheetNames = sf2MonthlySheets(workbook).map((sheet) => sheet.name);
	writeMarks(workbook, rosterNameMarks(sheetNames, assignments));

	const mappedRows = assignments.map((assignment) => assignment.slot.rowIndex);
	const expanded = extraMale > 0 || extraFemale > 0;
	const clearMarks = clearUnusedLearnerMarks(
		sheetNames,
		mappedRows,
		expanded ? maleCount : undefined,
		expanded ? femaleCount : undefined
	);
	if (clearMarks.length > 0) writeMarks(workbook, clearMarks);
	hideEmptyLearnerRows(workbook, rows.maleTotalRow, rows.femaleTotalRow, new Set(mappedRows));

	const dateMappings = dateMappingsFromAnalysis(template.id, analysis);
	for (const sheetName of mappedSheetNames(dateMappings)) clearTotalRows(workbook, sheetName, rows);

	// Best-effort, as in Rust: a formula that cannot be written must not cost the
	// teacher the roster they just saved.
	const marks = rosterSyncFormulaMarks(workbook, maleCount, femaleCount, rows, dateMappings);
	bestEffort(() => writeFormulaMarks(workbook, marks.totalMarks), 'TOTAL formula');
	bestEffort(() => writeFormulaMarks(workbook, marks.summaryFormulaMarks), 'summary formula');
	bestEffort(() => writeMarksForce(workbook, marks.summaryStaticMarks), 'summary static');

	return {
		analysis,
		studentMappings: studentMappingsFromRosterAssignments(template.id, assignments),
		studentsCreated: created,
		studentsReused: reused,
		learnersFound: students.length
	};
}

/**
 * A workbook the school handed over: the workbook's own learners are adopted onto the
 * class, and only the rows nothing occupies are emptied.
 */
async function adoptWorkbookRoster(
	workbook: Workbook,
	template: Sf2TemplateRecord,
	classId: string
): Promise<WorkbookOutcome> {
	const analysis = readWorkbookAnalysis(workbook);
	const sync = await syncWorkbookLearnerMappings(classId, template.id, analysis.learners);

	const mappedRows = sync.studentMappings.map((mapping) => mapping.rowIndex);
	const clearMarks = clearUnusedLearnerMarks(
		sf2MonthlySheets(workbook).map((sheet) => sheet.name),
		mappedRows
	);
	if (clearMarks.length > 0) writeMarks(workbook, clearMarks);
	// The TOTAL rows of a workbook the school laid out are its own, so they are read
	// from the template's own geometry rather than re-derived from a class count.
	hideEmptyLearnerRows(
		workbook,
		SF2_FRESH_MALE_TOTAL_ROW,
		SF2_FRESH_FEMALE_TOTAL_ROW,
		new Set(mappedRows)
	);

	return {
		analysis,
		studentMappings: sync.studentMappings,
		studentsCreated: sync.studentsCreated,
		studentsReused: sync.studentsReused,
		learnersFound: sync.studentMappings.length
	};
}

function countBlock(
	mappings: readonly Sf2StudentMappingRecord[],
	block: 'MALE' | 'FEMALE'
): number {
	return mappings.filter((mapping) => mapping.genderBlock === block).length;
}

function bestEffort(write: () => void, what: string): void {
	try {
		write();
	} catch (thrown) {
		const reason = thrown instanceof Error ? thrown.message : String(thrown);
		console.warn(`failed to write ${what} marks: ${reason}`);
	}
}
