import { readFile } from 'node:fs/promises';
import type { Workbook, Worksheet } from 'exceljs';
import { db, useTestDb } from '$lib/db/repos/__tests__/schema';
import { errorMessage } from '$lib/db';
import type { AppError } from '$lib/db';
import { loadTemplate, TEMPLATE_PATH } from '$lib/features/excel/__tests__/template-fixture';
import {
	getCellText,
	getCellTextAt,
	openWorkbook,
	sf2MonthlySheets,
	writableDayColumns
} from '$lib/features/excel/workbook';
import { useSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import { dateMappingsForTemplate, latestTemplateForClass } from '../repository';
import { createWorkbookFromTemplate, updateWorkbookSettings } from '../template';
import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import type { Sf2ImportSummary, Sf2TemplateDraft } from '$lib/types';

const WORKBOOK_DIR = '/workbooks';
const CLASS_ID = 'class-1';

/**
 * The pre-split tables and the settings row, in the shape the migration chain leaves
 * them. The shared repo fixture declares `sf2_student_mappings.id` as a primary key
 * the real table does not have, so both sf2 tables are recreated rather than patched.
 * `repository.test.ts` carries the same DDL for the same reason.
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

CREATE TABLE sf2_month_templates (
	id TEXT PRIMARY KEY NOT NULL,
	active_class_id TEXT NOT NULL,
	school_year TEXT NOT NULL,
	report_month TEXT NOT NULL,
	report_year INTEGER NOT NULL,
	source_path TEXT NOT NULL,
	source_hash TEXT NOT NULL,
	school_id TEXT,
	school_name TEXT,
	grade_level TEXT,
	section TEXT,
	adviser_name TEXT,
	school_head_name TEXT,
	first_school_day INTEGER NOT NULL,
	first_school_day_override INTEGER,
	imported_at INTEGER NOT NULL,
	last_synced_at INTEGER,
	workbook_x_count INTEGER NOT NULL DEFAULT 0,
	workbook_scanned_at INTEGER,
	UNIQUE(active_class_id, school_year, report_month)
);
`;

useTestDb();

function draft(overrides: Partial<Sf2TemplateDraft> = {}): Sf2TemplateDraft {
	return {
		classId: CLASS_ID,
		schoolId: '132839',
		schoolName: 'Espiritu Elementary School',
		schoolYear: '2026-2027',
		// SEPTEMBER 2026 starts on a Tuesday, so the 1st is the first school day.
		reportMonth: 'SEPTEMBER',
		firstSchoolDay: 1,
		gradeLevel: '3',
		section: 'MATAPAT',
		adviserName: 'DELA CRUZ, ADVISER',
		schoolHeadName: 'SANTOS, HEAD',
		learnerNames: [],
		...overrides
	};
}

async function seedClass(): Promise<void> {
	await db().script(SF2_SCHEMA);
	await db().execute(
		`INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
		 VALUES (?, 'Grade 3 - MATAPAT', '08:00', '15:00', '08:45', 1)`,
		[CLASS_ID]
	);
	for (const [id, name, gender] of [
		['s1', 'ALVARADO, ZYRON JAY  E.', 'male'],
		['s2', 'BAPTISMA, JONATHAN', 'male'],
		['s3', 'SALIMBOT, RAFA LATISHA', 'female']
	]) {
		await db().execute(
			'INSERT INTO students (id, name, gender, class_id, created_at) VALUES (?, ?, ?, ?, 1)',
			[id, name, gender, CLASS_ID]
		);
	}
}

/**
 * The bundled template ships as a Vite asset URL, which `fetch` cannot resolve in a
 * test process. Serving the real bytes from disk keeps `readBundledTemplate` on its
 * own code path rather than stubbing it out.
 */
async function serveBundledTemplate(): Promise<void> {
	const bytes = new Uint8Array(await readFile(TEMPLATE_PATH));
	globalThis.fetch = (async () => new Response(bytes)) as typeof fetch;
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

/** The worksheet the draft's report month lives on, which is the one that was rewritten. */
function septemberSheet(workbook: Workbook): Worksheet {
	const sheet = workbook.getWorksheet('SEPTEMBER 2026');
	if (sheet === undefined) throw new Error("the draft's report month is not in the workbook");
	return sheet;
}

/**
 * The first day column the calendar actually laid out.
 *
 * Not the first writable column: the grid merges day columns into pairs, so some
 * writable columns hold no day at all and are not a mapping anything can be written to.
 */
function firstMappedDay(sheet: Worksheet): string {
	const column = writableDayColumns(sheet).find(
		(letter) => getCellTextAt(sheet, `${letter}6`) !== ''
	);
	if (column === undefined) throw new Error('the worksheet has no day columns');
	return column;
}

let originalFetch: typeof globalThis.fetch;

beforeEach(async () => {
	originalFetch = globalThis.fetch;
	await serveBundledTemplate();
	useSf2WorkbookDir(WORKBOOK_DIR);
});

afterEach(() => {
	globalThis.fetch = originalFetch;
	useSf2WorkbookDir(null);
});

describe('createWorkbookFromTemplate', () => {
	beforeEach(async () => {
		await loadTemplate();
		await seedClass();
	});

	test('writes a working copy and records it with its roster and day grid', async () => {
		const summary = await createWorkbookFromTemplate(
			draft({
				learnerNames: ['ALVARADO, ZYRON JAY  E.', 'BAPTISMA, JONATHAN', 'SALIMBOT, RAFA LATISHA']
			})
		);

		expect(summary.classId).toBe(CLASS_ID);
		expect(summary.className).toBe('Grade 3 - MATAPAT');
		expect(summary.learnersFound).toBe(3);
		expect(summary.studentsReused).toBe(3);
		expect(summary.studentsCreated).toBe(0);
		expect(summary.datesMapped).toBeGreaterThan(0);

		const stored = await latestTemplateForClass(CLASS_ID);
		expect(stored?.sourceHash.startsWith('bundled-')).toBe(true);
		expect(stored?.sourcePath).toBe(summary.sourcePath);
		expect(stored?.layoutFingerprint).not.toBe('');

		const workbook = await openWorkbook(summary.sourcePath);
		const sheet = sf2MonthlySheets(workbook)[0];
		// 3 learners against a 21/19 template, so nothing is expanded.
		expect(getCellText(sheet!, 8, 3).trim()).toBe('ALVARADO, ZYRON JAY  E.');
		expect(getCellText(sheet!, 9, 3).trim()).toBe('BAPTISMA, JONATHAN');
		expect(getCellText(sheet!, 30, 3).trim()).toBe('SALIMBOT, RAFA LATISHA');
		// The template's own sample class is gone.
		expect(getCellText(sheet!, 10, 3).trim()).toBe('');
		expect(getCellTextAt(sheet!, 'C31').trim()).toBe('');
	});

	test('numbers the boys 1..n and the girls 1..m', async () => {
		const summary = await createWorkbookFromTemplate(
			draft({
				learnerNames: ['ALVARADO, ZYRON JAY  E.', 'BAPTISMA, JONATHAN', 'SALIMBOT, RAFA LATISHA']
			})
		);
		const sheet = sf2MonthlySheets(await openWorkbook(summary.sourcePath))[0]!;
		expect(getCellText(sheet, 8, 1)).toBe('1');
		expect(getCellText(sheet, 9, 1)).toBe('2');
		expect(getCellText(sheet, 30, 1)).toBe('1');
	});

	test('hides the learner rows nothing sits in, so only the class prints', async () => {
		const summary = await createWorkbookFromTemplate(
			draft({ learnerNames: ['ALVARADO, ZYRON JAY  E.'] })
		);
		const sheet = sf2MonthlySheets(await openWorkbook(summary.sourcePath))[0]!;
		expect(sheet.getRow(8).hidden).toBe(false);
		expect(sheet.getRow(9).hidden).toBe(true);
		expect(sheet.getRow(30).hidden).toBe(true);
	});

	test('writes the header block onto the form', async () => {
		const summary = await createWorkbookFromTemplate(draft());
		const sheet = sf2MonthlySheets(await openWorkbook(summary.sourcePath))[0]!;
		expect(getCellText(sheet, 3, 6).trim()).toBe('132839');
		expect(getCellText(sheet, 4, 6).trim()).toBe('Espiritu Elementary School');
	});

	test('writes the TOTAL Per Day formulas, with their cached values', async () => {
		const summary = await createWorkbookFromTemplate(draft());
		const sheet = septemberSheet(await openWorkbook(summary.sourcePath));
		const firstDay = firstMappedDay(sheet);
		// 2 boys, 1 girl, nobody absent yet, so the first day's MALE TOTAL is 2.
		expect(getCellTextAt(sheet, `${firstDay}29`)).toBe('2');
		expect(getCellTextAt(sheet, `${firstDay}49`)).toBe('1');
		expect(getCellTextAt(sheet, `${firstDay}50`)).toBe('3');
		expect(sheet.getCell(`${firstDay}29`).model.formula).toBe(
			`2-COUNTIF(${firstDay}8:${firstDay}28,"X")`
		);
	});

	test('records the enrolment on row 53', async () => {
		const summary = await createWorkbookFromTemplate(draft());
		const sheet = septemberSheet(await openWorkbook(summary.sourcePath));
		expect(getCellTextAt(sheet, 'AR53')).toBe('2');
		expect(getCellTextAt(sheet, 'AS53')).toBe('1');
		expect(getCellTextAt(sheet, 'AT53')).toBe('3');
	});

	test('refuses a class that already has a workbook', async () => {
		await createWorkbookFromTemplate(draft());
		expect(await failure(createWorkbookFromTemplate(draft()))).toMatch(
			/An SF2 workbook already exists for Grade 3 - MATAPAT/
		);
	});

	test('creates the class from the grade level and section when the draft names none', async () => {
		const summary = await createWorkbookFromTemplate(draft({ classId: undefined }));
		expect(summary.className).toBe('3 - MATAPAT');
		const created = await db().queryOne<{ name: string }>('SELECT name FROM classes WHERE id = ?', [
			summary.classId
		]);
		expect(created?.name).toBe('3 - MATAPAT');
	});

	test('refuses a learner the school has given no gender for', async () => {
		// A draft name the class has never seen becomes a student with no gender, and
		// the form has one block per gender and no third block - so the whole create
		// is refused rather than dropping the child.
		expect(
			await failure(createWorkbookFromTemplate(draft({ learnerNames: ['DELA CRUZ, JUAN'] })))
		).toMatch(/Set Male\/Female.*DELA CRUZ, JUAN/);
	});

	test('reuses one student when the draft spells a name twice', async () => {
		// `rosterStudentsForDraft` de-duplicates by normalized name, so a doubled
		// entry cannot become two students - which is why the duplicate-name guard
		// matters on the roster-sync path, where the whole class is read at once.
		const summary = await createWorkbookFromTemplate(
			draft({ learnerNames: ['ALVARADO, ZYRON JAY  E.', 'alvarado, zyron jay e.'] })
		);
		expect(summary.studentsReused).toBe(1);
		expect(summary.learnersFound).toBe(1);
		const count = await db().queryOne<{ count: number }>(
			'SELECT COUNT(*) AS count FROM students WHERE class_id = ?',
			[CLASS_ID]
		);
		expect(Number(count?.count)).toBe(3);
	});

	test('refuses a first attendance day the month cannot hold', async () => {
		expect(await failure(createWorkbookFromTemplate(draft({ firstSchoolDay: 5 })))).toMatch(
			/First attendance day must be a Monday-Friday school day/
		);
	});
});

describe('updateWorkbookSettings', () => {
	beforeEach(async () => {
		await loadTemplate();
		await seedClass();
	});

	async function createOne(): Promise<Sf2ImportSummary> {
		return createWorkbookFromTemplate(draft({ learnerNames: ['ALVARADO, ZYRON JAY  E.'] }));
	}

	test('re-lays out a bundled workbook and reports the same class', async () => {
		const created = await createOne();
		const summary = await updateWorkbookSettings(
			draft({
				schoolName: 'Espiritu Elementary School',
				learnerNames: ['ALVARADO, ZYRON JAY  E.', 'BAPTISMA, JONATHAN']
			})
		);

		expect(summary.templateId).toBe(created.templateId);
		expect(summary.learnersFound).toBe(2);
		expect(summary.classId).toBe(CLASS_ID);

		const sheet = sf2MonthlySheets(await openWorkbook(created.sourcePath))[0]!;
		expect(getCellText(sheet, 8, 3).trim()).toBe('ALVARADO, ZYRON JAY  E.');
		expect(getCellText(sheet, 9, 3).trim()).toBe('BAPTISMA, JONATHAN');
		expect(getCellText(sheet, 10, 3).trim()).toBe('');
	});

	test('rewrites the header block and the TOTAL row counts', async () => {
		const created = await createOne();
		await updateWorkbookSettings(
			draft({ schoolName: 'A New School Name', learnerNames: ['ALVARADO, ZYRON JAY  E.'] })
		);
		const sheet = septemberSheet(await openWorkbook(created.sourcePath));
		expect(getCellText(sheet, 4, 6).trim()).toBe('A New School Name');
		const firstDay = firstMappedDay(sheet);
		expect(getCellTextAt(sheet, `${firstDay}29`)).toBe('1');
		expect(getCellTextAt(sheet, 'AR53')).toBe('1');
	});

	test('carries edited details onto the class month rows', async () => {
		// Opening a month stamps the sheet from its *month* row, so an edit that
		// stops at the legacy row is stamped straight back to the stale names on
		// the next open (blank fields even clear the signature cells).
		await createOne();
		const seedMonth = (id: string, classId: string, schoolYear: string, month: string) =>
			db().execute(
				`INSERT INTO sf2_month_templates (
					id, active_class_id, school_year, report_month, report_year,
					source_path, source_hash, school_id, school_name, grade_level,
					section, adviser_name, school_head_name, first_school_day, imported_at)
				 VALUES (?, ?, ?, ?, 2026, '/workbooks/f.xlsx', 'h', 'old-id', 'Old School',
				         '3', 'MATAPAT', 'OLD ADVISER', 'OLD HEAD', 1, 1)`,
				[id, classId, schoolYear, month]
			);
		await seedMonth('m-sept', CLASS_ID, '2026-2027', 'SEPTEMBER');
		await seedMonth('m-oct', CLASS_ID, '2026-2027', 'OCTOBER');
		await seedMonth('m-other-class', 'class-9', '2026-2027', 'SEPTEMBER');
		await seedMonth('m-other-year', CLASS_ID, '2025-2026', 'SEPTEMBER');

		await updateWorkbookSettings(
			draft({
				adviserName: 'NEW ADVISER',
				schoolHeadName: 'NEW HEAD',
				learnerNames: ['ALVARADO, ZYRON JAY  E.']
			})
		);

		const names = await db().query<{ id: string; adviser_name: string | null }>(
			'SELECT id, adviser_name FROM sf2_month_templates ORDER BY id'
		);
		const byId = new Map(names.map((row) => [row.id, row.adviser_name]));
		expect(byId.get('m-sept')).toBe('NEW ADVISER');
		expect(byId.get('m-oct')).toBe('NEW ADVISER');
		expect(byId.get('m-other-class')).toBe('OLD ADVISER');
		expect(byId.get('m-other-year')).toBe('OLD ADVISER');

		const heads = await db().query<{ id: string; school_head_name: string | null }>(
			'SELECT id, school_head_name FROM sf2_month_templates ORDER BY id'
		);
		expect(new Map(heads.map((row) => [row.id, row.school_head_name])).get('m-sept')).toBe(
			'NEW HEAD'
		);
		// Month dating is not metadata: the refresh must not touch it.
		const dating = await db().queryOne<{ report_month: string; first_school_day: number }>(
			'SELECT report_month, first_school_day FROM sf2_month_templates WHERE id = ?',
			['m-sept']
		);
		expect(dating?.report_month).toBe('SEPTEMBER');
		expect(Number(dating?.first_school_day)).toBe(1);
	});

	test('refuses when the class has no workbook', async () => {
		expect(await failure(updateWorkbookSettings(draft()))).toMatch(
			/No SF2 workbook imported for this class/
		);
	});

	test('refuses a draft with no class', async () => {
		expect(await failure(updateWorkbookSettings(draft({ classId: '  ' })))).toMatch(
			/Class is required/
		);
	});

	test('refuses when the workbook file has gone', async () => {
		const created = await createOne();
		await db().execute('UPDATE sf2_templates SET source_path = ? WHERE id = ?', [
			'/workbooks/gone.xlsx',
			created.templateId
		]);
		expect(await failure(updateWorkbookSettings(draft()))).toMatch(
			/The app SF2 working workbook no longer exists/
		);
	});

	test('refuses when the class itself has gone', async () => {
		await createOne();
		await db().execute('DELETE FROM classes WHERE id = ?', [CLASS_ID]);
		expect(await failure(updateWorkbookSettings(draft()))).toMatch(/Selected class was not found/);
	});

	test('adopts a workbook the school handed over rather than re-laying it out', async () => {
		// A source hash without the `bundled-` prefix means the workbook is the school's:
		// its rows are its arrangement and its learners become the class's students.
		const created = await createOne();
		await db().execute('UPDATE sf2_templates SET source_hash = ? WHERE id = ?', [
			'school-hash',
			created.templateId
		]);
		const summary = await updateWorkbookSettings(draft());

		expect(summary.learnersFound).toBeGreaterThan(0);
		const rows = await db().query<{ row_index: number; gender_block: string | null }>(
			'SELECT row_index, gender_block FROM sf2_student_mappings WHERE template_id = ? ORDER BY row_index',
			[created.templateId]
		);
		// The workbook's own learners, in the rows they already occupied.
		expect(rows.length).toBe(summary.learnersFound);
		expect(rows[0].row_index).toBe(8);
	});

	test('a re-analysis of an imported workbook never records an empty grid', async () => {
		const created = await createOne();
		await db().execute('UPDATE sf2_templates SET source_hash = ? WHERE id = ?', [
			'school-hash',
			created.templateId
		]);
		await updateWorkbookSettings(draft());
		expect(await dateMappingsForTemplate(created.templateId)).not.toEqual([]);
	});
});
