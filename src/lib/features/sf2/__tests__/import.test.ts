import { db, useTestDb } from '$lib/db/repos/__tests__/schema';
import { errorMessage } from '$lib/db';
import type { AppError } from '$lib/db';
import { loadTemplate, newWorkbook } from '$lib/features/excel/__tests__/template-fixture';
import { workbookToBytes, openWorkbook } from '$lib/features/excel/workbook';
import { getFileSystem } from '$lib/platform/fs';
import { useSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import { className } from '$lib/features/sf2/naming';
import { readWorkbookAnalysis } from '$lib/features/sf2/roster/analysis';
import { latestTemplateForClass } from '../repository';
import { importSf2WorkbookFromFile, stageImportSource } from '../template/import';
import { afterEach, beforeEach, describe, expect, test } from 'vitest';

const WORKBOOK_DIR = '/workbooks';
const PICKED_PATH = '/picked/school-file.xlsx';

/**
 * The `sf2_*` tables plus settings, in the shape the migration chain leaves
 * them. Copied from `template.test.ts`, which carries the same DDL because the
 * shared repo fixture declares a `sf2_student_mappings.id` primary key the real
 * table does not have.
 */
const SF2_SCHEMA = `
CREATE TABLE settings (
	id TEXT PRIMARY KEY NOT NULL,
	day_start TEXT NOT NULL,
	day_end TEXT NOT NULL,
	late_after TEXT NOT NULL,
	quarter TEXT NOT NULL DEFAULT '1st Quarter',
	q1_start TEXT,
	q1_end TEXT,
	q2_start TEXT,
	q2_end TEXT,
	q3_start TEXT,
	q3_end TEXT,
	attendance_mode TEXT NOT NULL DEFAULT 'manual',
	school_id TEXT,
	school_name TEXT,
	school_year TEXT,
	report_month TEXT,
	grade_level TEXT,
	section TEXT,
	adviser_name TEXT,
	school_head_name TEXT,
	branding_title TEXT
);

DROP TABLE sf2_student_mappings;
CREATE TABLE sf2_student_mappings (
	template_id TEXT NOT NULL,
	student_id TEXT NOT NULL,
	workbook_name TEXT NOT NULL,
	normalized_name TEXT NOT NULL,
	row_index INTEGER NOT NULL,
	gender_block TEXT,
	PRIMARY KEY (template_id, student_id)
);

DROP TABLE sf2_templates;
CREATE TABLE sf2_templates (
	id TEXT PRIMARY KEY NOT NULL,
	source_path TEXT,
	source_hash TEXT NOT NULL DEFAULT '',
	school_id TEXT,
	school_name TEXT,
	school_year TEXT,
	report_month TEXT,
	grade_level TEXT,
	section TEXT,
	adviser_name TEXT,
	school_head_name TEXT,
	layout_fingerprint TEXT,
	active_class_id TEXT,
	imported_at INTEGER NOT NULL,
	last_synced_at INTEGER,
	UNIQUE (source_hash, grade_level, section)
);

CREATE TABLE sf2_date_mappings (
	template_id TEXT NOT NULL,
	sheet_name TEXT,
	date TEXT NOT NULL,
	column_letter TEXT NOT NULL,
	column_index INTEGER NOT NULL,
	PRIMARY KEY (template_id, date)
);
`;

useTestDb();

async function seedSchema(): Promise<void> {
	await db().script(SF2_SCHEMA);
}

async function seedClass(id: string, name: string, students: string[]): Promise<void> {
	await db().execute(
		`INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
		 VALUES (?, ?, '08:00', '15:00', '08:45', 1)`,
		[id, name]
	);
	for (const [index, student] of students.entries()) {
		await db().execute(
			'INSERT INTO students (id, name, gender, class_id, created_at) VALUES (?, ?, ?, ?, 1)',
			[`s${index}`, student, 'male', id]
		);
	}
}

/** The real template bytes, posed as a teacher-picked file outside the app folder. */
async function pickRealTemplate(pickedPath = PICKED_PATH): Promise<void> {
	const fixture = await loadTemplate();
	const bytes = await workbookToBytes(await fixture.open());
	await getFileSystem().writeFileAtomic(pickedPath, bytes);
}

async function failure(promise: Promise<unknown>): Promise<string> {
	try {
		await promise;
	} catch (thrown) {
		return errorMessage(thrown as AppError);
	}
	expect.unreachable('the call was expected to fail');
	return '';
}

beforeEach(async () => {
	useSf2WorkbookDir(WORKBOOK_DIR);
});

afterEach(() => {
	useSf2WorkbookDir(null);
});

describe('importSf2WorkbookFromFile', () => {
	beforeEach(async () => {
		await seedSchema();
		await pickRealTemplate();
	});

	test('imports a staged workbook: adopts its roster and records the template', async () => {
		const staged = await stageImportSource(PICKED_PATH);
		const summary = await importSf2WorkbookFromFile(staged, false);

		expect(summary.learnersFound).toBeGreaterThan(0);
		expect(summary.datesMapped).toBeGreaterThan(0);
		expect(summary.sourcePath).toContain('/workbooks/SF2-');

		const latest = await latestTemplateForClass(summary.classId);
		expect(latest?.id).toBe(summary.templateId);
	});

	test('refuses an old .xls file with the Save-As message', async () => {
		expect(await failure(stageImportSource('/picked/old.xls'))).toMatch(/Save As \.xlsx/);
	});

	test('refuses a roster mismatch unless the teacher proceeds', async () => {
		const analysis = readWorkbookAnalysis(await openWorkbook(PICKED_PATH));
		await seedClass('class-9', className(analysis.gradeLevel, analysis.section), [
			'NOT A REAL LEARNER, ANYONE'
		]);

		const staged = await stageImportSource(PICKED_PATH);
		expect(await failure(importSf2WorkbookFromFile(staged, false))).toMatch(
			/Student List Mismatch Detected/
		);

		const summary = await importSf2WorkbookFromFile(await stageImportSource(PICKED_PATH), true);
		expect(summary.learnersFound).toBeGreaterThan(0);
	});

	test('refuses a second import for a class that has a workbook', async () => {
		await importSf2WorkbookFromFile(await stageImportSource(PICKED_PATH), true);
		expect(
			await failure(importSf2WorkbookFromFile(await stageImportSource(PICKED_PATH), true))
		).toMatch(/already exists for/);
	});

	test('refuses a workbook with no learners', async () => {
		await getFileSystem().writeFileAtomic(
			'/picked/empty.xlsx',
			await workbookToBytes(newWorkbook('JUNE 2025'))
		);
		expect(
			await failure(importSf2WorkbookFromFile(await stageImportSource('/picked/empty.xlsx'), true))
		).toMatch(/no learners|no calendar dates/i);
	});
});
