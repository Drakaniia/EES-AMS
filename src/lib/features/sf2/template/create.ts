import { invalidInput } from '$lib/db';
import { getClass } from '$lib/db/repos/classes';
import { getSettings } from '$lib/db/repos/settings';
import { nowEpochSeconds } from '$lib/domain/models';
import {
	clearTotalRows,
	writeFormulaMarks,
	writeMarks,
	writeMarksForce,
	writeMetadata
} from '$lib/features/excel/marks';
import { expandRosterRows, hideEmptyLearnerRows } from '$lib/features/excel/roster';
import {
	getCellTextAt,
	openWorkbook,
	saveWorkbookAtomic,
	sf2MonthlySheets,
	writableDayColumns
} from '$lib/features/excel/workbook';
import { SF2_FIRST_LEARNER_ROW } from '$lib/features/excel/constants';
import { configureSf2Calendar, validateConfiguredCalendar } from '../calendar';
import type { Sf2TemplateMetadata } from '../calendar';
import { className } from '../naming';
import { dateMappingsFromAnalysis, metadataFromDraft } from '../metadata';
import {
	hashBytes,
	layoutFingerprint,
	readBundledTemplate,
	writeBundledTemplateToDir
} from '../workbook-files';
import { findTemplate, latestTemplateForClass, upsertTemplateWithMappings } from '../repository';
import { readWorkbookAnalysis } from '../roster/analysis';
import { clearUnusedLearnerMarks, findOrCreateClass, genderCounts } from '../roster/helpers';
import { mappedSheetNames, rosterSyncFormulaMarks } from '../roster/formula-marks';
import { rosterStudentsForDraft } from '../roster/learner-sync';
import {
	bundledTemplateTotalRows,
	rejectDuplicateRosterNames,
	rosterExpansionNeeded,
	rosterNameMarks,
	studentMappingsFromRosterAssignments,
	templateRosterAssignments
} from '../roster/parser';
import type { Sf2ImportSummary, Sf2TemplateDraft } from '$lib/types';
import type { Workbook } from 'exceljs';
import type { Sf2CellMark } from '$lib/features/excel/types';
import type { Class } from '$lib/domain/models';
import type { Sf2TemplateRecord } from '../repository';

/**
 * Create a class's SF2 workbook from the bundled DepEd template.
 *
 * The whole build happens on one in-memory workbook that is written once.
 */

/**
 * The class the new workbook belongs to: the one the draft names, or one derived
 * from the grade level and section cells and created if it does not exist.
 */
async function resolveTemplateClass(
	draft: Sf2TemplateDraft,
	gradeLevel: string,
	section: string
): Promise<Class> {
	const asked = draft.classId?.trim();
	if (asked !== undefined && asked !== '') {
		const found = await getClass(asked);
		if (found === undefined) throw invalidInput('Selected class was not found');
		return found;
	}
	return findOrCreateClass(className(gradeLevel, section), await getSettings());
}

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

/**
 * Every non-blank cell of every month sheet's day grid, as a clear.
 *
 * The bundled template ships DepEd's own sample `X` marks. A workbook written from
 * it must not keep them: they are counted into the ABSENT totals the teacher reads,
 * and the split that follows reads them as absences the database never recorded and
 * refuses to write a single month row - which is what leaves Reports saying no
 * workbook exists for the month that was just created.
 */
function clearSampleDayMarks(workbook: Workbook, lastLearnerRow: number): Sf2CellMark[] {
	const marks: Sf2CellMark[] = [];
	for (const sheet of sf2MonthlySheets(workbook)) {
		for (const column of writableDayColumns(sheet)) {
			for (let row = SF2_FIRST_LEARNER_ROW; row <= lastLearnerRow; row += 1) {
				const address = `${column}${row}`;
				if (getCellTextAt(sheet, address).trim() === '') continue;
				marks.push({ sheetName: sheet.name, address, value: '' });
			}
		}
	}
	return marks;
}

/**
 * Write a new working copy of the bundled template and record it.
 *
 * A month starts from the template rather than from the previous month's file,
 * because cloning that file would put one month's `X` marks in another.
 */
export async function createWorkbookFromTemplate(
	draft: Sf2TemplateDraft
): Promise<Sf2ImportSummary> {
	const metadata = metadataFromDraft(draft);
	const classRecord = await resolveTemplateClass(draft, metadata.gradeLevel, metadata.section);

	const existing = await latestTemplateForClass(classRecord.id);
	if (existing !== undefined) {
		throw invalidInput(
			`An SF2 workbook already exists for ${classRecord.name}. ` +
				'Update the existing workbook settings instead of creating a new one'
		);
	}

	const { students, created, reused } = await rosterStudentsForDraft(
		classRecord.id,
		draft.learnerNames
	);
	rejectDuplicateRosterNames(students);

	const assignments = templateRosterAssignments(students);
	const { maleCount, femaleCount } = genderCounts(students);
	const { extraMale, extraFemale } = rosterExpansionNeeded(maleCount, femaleCount);
	const rows = bundledTemplateTotalRows(maleCount, femaleCount);

	// The `bundled-` prefix is what tells a roster sync the app owns this workbook's
	// rows, and the class id keeps two classes from sharing one working copy.
	const sourceHash = `bundled-${hashBytes(await readBundledTemplate())}-${classRecord.id}`;
	const templateId =
		(await findTemplate(sourceHash, metadata.gradeLevel, metadata.section))?.id ??
		crypto.randomUUID();
	const workingCopyPath = await writeBundledTemplateToDir({
		templateId,
		gradeLevel: metadata.gradeLevel,
		section: metadata.section
	});

	// Pure arithmetic, no workbook: the mappings are known before the file is opened.
	const studentMappings = studentMappingsFromRosterAssignments(templateId, assignments);

	const workbook = await openWorkbook(workingCopyPath);
	writeMetadata(workbook, headerBlock(metadata));
	// The header and the calendar are one write in Rust: a template whose day row
	// still holds the DepEd sample's 2025 dates would fail its own calendar check.
	configureSf2Calendar(workbook, metadata);
	if (extraMale > 0 || extraFemale > 0) expandRosterRows(workbook, extraMale, extraFemale);

	const sampleMarks = clearSampleDayMarks(workbook, rows.femaleTotalRow - 1);
	if (sampleMarks.length > 0) writeMarks(workbook, sampleMarks);

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

	const dateMappings = dateMappingsFromAnalysis(templateId, analysis);
	for (const sheetName of mappedSheetNames(dateMappings)) clearTotalRows(workbook, sheetName, rows);

	const marks = rosterSyncFormulaMarks(workbook, maleCount, femaleCount, rows, dateMappings);
	writeFormulaMarks(workbook, marks.totalMarks);
	writeFormulaMarks(workbook, marks.summaryFormulaMarks);
	writeMarksForce(workbook, marks.summaryStaticMarks);

	await saveWorkbookAtomic(workbook, workingCopyPath);

	// Checked after the write, as in Rust: the calendar is only known to be right once
	// the workbook has said which days it laid out.
	validateConfiguredCalendar(analysis, metadata);

	const template: Sf2TemplateRecord = {
		id: templateId,
		sourcePath: workingCopyPath,
		sourceHash,
		schoolId: metadata.schoolId,
		schoolName: metadata.schoolName,
		schoolYear: metadata.schoolYear,
		reportMonth: metadata.reportMonth,
		gradeLevel: metadata.gradeLevel,
		section: metadata.section,
		adviserName: metadata.adviserName,
		schoolHeadName: metadata.schoolHeadName,
		layoutFingerprint: layoutFingerprint(analysis),
		activeClassId: classRecord.id,
		importedAt: nowEpochSeconds(),
		lastSyncedAt: undefined
	};
	await upsertTemplateWithMappings(template, studentMappings, dateMappings);

	return {
		templateId,
		classId: classRecord.id,
		className: classRecord.name,
		sourcePath: workingCopyPath,
		schoolYear: template.schoolYear,
		gradeLevel: template.gradeLevel,
		section: template.section,
		learnersFound: students.length,
		studentsCreated: created,
		studentsReused: reused,
		datesMapped: dateMappings.length
	};
}
