import { invalidInput } from '$lib/db';
import { getSettings } from '$lib/db/repos/settings';
import { nowEpochSeconds } from '$lib/domain/models';
import { baseName } from '$lib/features/backup/paths';
import { openWorkbook } from '$lib/features/excel/workbook';
import { getFileSystem } from '$lib/platform/fs';
import type { Sf2ImportSummary } from '$lib/types';
import { readWorkbookAnalysis } from '../roster/analysis';
import { findOrCreateClass } from '../roster/helpers';
import { syncWorkbookLearnerMappings } from '../roster/learner-sync';
import { className } from '../naming';
import { dateMappingsFromAnalysis, metadataFromImportAnalysis } from '../metadata';
import {
	EMPTY_DATE_ANALYSIS_MESSAGE,
	EMPTY_ROSTER_ANALYSIS_MESSAGE,
	latestTemplateForClass,
	upsertTemplateWithMappings
} from '../repository';
import { ensureImportValidationAllows, type Sf2ImportValidation } from '../validation';
import { importValidationFromAnalysis } from '../validation-service';
import {
	getSf2WorkbookDir,
	hashBytes,
	layoutFingerprint,
	singleWorkbookPath
} from '../workbook-files';

/**
 * Adopting a school's workbook as the class working copy — the TypeScript
 * successor to the removed `import_sf2_workbook` command (spec D18 reversal).
 *
 * The picked file is staged inside the workbook directory first: the fs
 * capability is a static allow-list (see `src/lib/api/restore-staging.ts`),
 * so a file the teacher chose anywhere else is staged somewhere in-scope
 * before anything reads it. The staged copy then becomes the working copy via
 * `rename`, so there is only ever one file, never a staged duplicate beside
 * the record pointing at it.
 */

const STAGING_FOLDER = 'import-staging';

/**
 * Copy a teacher-picked file somewhere the fs allow-list covers.
 *
 * Old `.xls` files are refused, not converted: an `.xls` renamed to `.xlsx`
 * is still an `.xls` inside, and only Excel's own Save As produces a real
 * `.xlsx` matching the bundled template's layout.
 */
export async function stageImportSource(pickedPath: string): Promise<string> {
	const lower = pickedPath.toLowerCase();
	if (lower.endsWith('.xls') && !lower.endsWith('.xlsx')) {
		throw invalidInput(
			'This is an old .xls workbook. Open it once in Excel, Save As .xlsx, then import from that file.'
		);
	}
	const fs = getFileSystem();
	const bytes = await fs.readFile(pickedPath);
	const dir = `${await getSf2WorkbookDir()}/${STAGING_FOLDER}`;
	await fs.mkdirp(dir);
	for (const name of await fs.readDir(dir)) {
		if (name.endsWith('/')) continue;
		await fs.remove(`${dir}/${name}`);
	}
	const path = `${dir}/import-${baseName(pickedPath)}`;
	await fs.writeFileAtomic(path, bytes);
	return path;
}

/** Analyze a staged file and report roster disagreements without writing anything. */
export async function validateSf2WorkbookImport(stagedPath: string): Promise<Sf2ImportValidation> {
	return importValidationFromAnalysis(
		stagedPath,
		readWorkbookAnalysis(await openWorkbook(stagedPath))
	);
}

/**
 * Adopt a staged workbook as the class working copy.
 *
 * The roster is the school's arrangement and is never re-laid-out: learners
 * become students via `syncWorkbookLearnerMappings`, and the source hash
 * deliberately lacks the `bundled-` prefix so `templateOwnsRoster` treats it
 * as the school's file from here on.
 */
export async function importSf2WorkbookFromFile(
	stagedPath: string,
	proceedAnyway: boolean
): Promise<Sf2ImportSummary> {
	const fs = getFileSystem();
	const bytes = await fs.readFile(stagedPath);
	const analysis = readWorkbookAnalysis(await openWorkbook(stagedPath));

	const validation = await importValidationFromAnalysis(stagedPath, analysis);
	ensureImportValidationAllows(validation, proceedAnyway);
	if (analysis.dates.length === 0) throw invalidInput(EMPTY_DATE_ANALYSIS_MESSAGE);

	const metadata = metadataFromImportAnalysis(analysis);
	const classRecord = await findOrCreateClass(
		className(metadata.gradeLevel, metadata.section),
		await getSettings()
	);
	const existing = await latestTemplateForClass(classRecord.id);
	if (existing !== undefined) {
		throw invalidInput(
			`An SF2 workbook already exists for ${classRecord.name}. ` +
				'Update the existing workbook settings instead of creating a new one'
		);
	}

	const templateId = crypto.randomUUID();
	const sync = await syncWorkbookLearnerMappings(classRecord.id, templateId, analysis.learners);
	if (sync.studentMappings.length === 0) throw invalidInput(EMPTY_ROSTER_ANALYSIS_MESSAGE);
	const dateMappings = dateMappingsFromAnalysis(templateId, analysis);

	const sourcePath = singleWorkbookPath(await getSf2WorkbookDir(), {
		templateId,
		gradeLevel: metadata.gradeLevel,
		section: metadata.section
	});
	await fs.rename(stagedPath, sourcePath);

	const template = {
		id: templateId,
		sourcePath,
		sourceHash: `imported-${hashBytes(bytes)}-${classRecord.id}`,
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
	await upsertTemplateWithMappings(template, sync.studentMappings, dateMappings);

	return {
		templateId,
		classId: classRecord.id,
		className: classRecord.name,
		sourcePath,
		schoolYear: template.schoolYear,
		gradeLevel: template.gradeLevel,
		section: template.section,
		learnersFound: sync.studentMappings.length,
		studentsCreated: sync.studentsCreated,
		studentsReused: sync.studentsReused,
		datesMapped: dateMappings.length
	};
}
