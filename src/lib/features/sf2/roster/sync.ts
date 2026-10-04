import { invalidInput } from '$lib/db';
import { getClass } from '$lib/db/repos/classes';
import { listStudents } from '$lib/db/repos/students';
import { getFileSystem } from '$lib/platform/fs';
import { latestTemplateForClass, studentMappingsForTemplate } from '../repository';
import { syncBundledTemplateRoster, syncImportedWorkbookRoster } from './internal';
import { rejectDuplicateRosterNames, templateOwnsRoster } from './parser';
import type { Sf2TemplateRecord } from '../repository';

/**
 * Re-point a class's workbook at the class's current roster — the port of
 * `src-tauri/src/sf2/roster/roster_sync.rs`.
 *
 * This is what every add / update / delete-student command runs afterwards, so it is
 * on the hot path of the Students page rather than a rare operation.
 */

/** Sync the newest pre-split workbook of a class, if it has one. */
export async function syncWorkbookRosterForClass(classId: string): Promise<void> {
	await syncLatestWorkbookRosterForClass(classId);
}

/**
 * Sync the newest workbook of a class and hand the refreshed template row back.
 *
 * `undefined` when the class has no pre-split workbook at all, which is the normal
 * state on the per-month model: a month is a worksheet, not a template row.
 */
export async function syncLatestWorkbookRosterForClass(
	classId: string
): Promise<Sf2TemplateRecord | undefined> {
	const template = await latestTemplateForClass(classId);
	if (template === undefined) return undefined;
	return await syncTemplateRosterFromClass(template);
}

/**
 * Re-point one workbook at its class's current roster.
 *
 * The workbook has to be on disk. A row pointing at a file that is gone is worse than
 * an error, because every later mark write would land nowhere and the teacher would
 * find out from a printed report.
 */
export async function syncTemplateRosterFromClass(
	template: Sf2TemplateRecord
): Promise<Sf2TemplateRecord> {
	if (!(await getFileSystem().exists(template.sourcePath))) {
		throw invalidInput(
			'The app SF2 working workbook no longer exists. Import the SF2 workbook again'
		);
	}

	const classRecord = await getClass(template.activeClassId);
	if (classRecord === undefined) throw invalidInput('Selected class was not found');
	const students = await listStudents(classRecord.id);
	rejectDuplicateRosterNames(students);

	const existingMappings = await studentMappingsForTemplate(template.id);
	return templateOwnsRoster(template)
		? await syncBundledTemplateRoster(template, students, existingMappings)
		: await syncImportedWorkbookRoster(template, students, existingMappings);
}
