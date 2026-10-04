import { describe, expect, test } from 'vitest';
import { db, insertMonthTemplate, useMonthTestDb } from './schema';
import { loadTemplate, WORKBOOK_PATH } from '$lib/features/excel/__tests__/template-fixture';
import { getCellTextAt, openWorkbook } from '$lib/features/excel/workbook';
import { syncMonthRosterForClass } from '../roster-sync';
import type { Worksheet } from 'exceljs';

useMonthTestDb();

const CLASS_ID = 'class-1';

async function addStudent(id: string, name: string, gender: 'male' | 'female'): Promise<void> {
	await db().execute(
		'INSERT INTO students (id, name, gender, class_id, created_at) VALUES (?, ?, ?, ?, 1)',
		[id, name, gender, CLASS_ID]
	);
}

async function seedMapping(studentId: string, name: string, rowIndex: number, block: string) {
	await db().execute(
		`INSERT INTO sf2_month_student_mappings
		   (template_id, student_id, workbook_name, normalized_name, row_index, gender_block)
		 VALUES ('month-1', ?, ?, ?, ?, ?)`,
		[studentId, name, name, rowIndex, block]
	);
}

/** One month on record, on the bundled template's own June worksheet. */
async function seedJuneMonth(): Promise<void> {
	await insertMonthTemplate({
		id: 'month-1',
		classId: CLASS_ID,
		sourcePath: WORKBOOK_PATH,
		reportMonth: 'JUNE',
		reportYear: 2025
	});
	await loadTemplate();
}

/** The saved workbook's June worksheet, as the teacher would see it. */
async function juneSheet(): Promise<Worksheet> {
	const sheet = (await openWorkbook(WORKBOOK_PATH)).getWorksheet('JUNE 2025');
	if (sheet === undefined) throw new Error('the fixture has no JUNE 2025 worksheet');
	return sheet;
}

async function mappings() {
	return await db().query<{ student_id: string; row_index: number; gender_block: string }>(
		'SELECT student_id, row_index, gender_block FROM sf2_month_student_mappings ORDER BY row_index'
	);
}

describe('syncMonthRosterForClass', () => {
	test('a class with no month on record maps nothing and is not an error', async () => {
		await loadTemplate();
		expect(await syncMonthRosterForClass(CLASS_ID)).toBe(0);
	});

	test('a student added after the split is given a free row and written onto the sheet', async () => {
		await seedJuneMonth();
		await addStudent('s1', 'ANA', 'male');
		await addStudent('s2', 'BEA', 'female');
		await seedMapping('s1', 'ANA', 8, 'MALE');

		expect(await syncMonthRosterForClass(CLASS_ID)).toBe(2);

		// ANA keeps row 8 - her X marks live there - and BEA takes the first free
		// row of the female block.
		expect(await mappings()).toEqual([
			{ student_id: 's1', row_index: 8, gender_block: 'MALE' },
			{ student_id: 's2', row_index: 30, gender_block: 'FEMALE' }
		]);

		const sheet = juneSheet();
		expect(getCellTextAt(await sheet, 'C30')).toBe('BEA');
		// The learner below her was vacated by the delete, so its cells are blank.
		expect(getCellTextAt(await sheet, 'C31')).toBe('');
	});

	test('a gender block that has run out of rows grows rather than dropping the student', async () => {
		await seedJuneMonth();
		for (let index = 0; index < 22; index += 1) {
			await addStudent(`m${index}`, `MAN ${String(index).padStart(2, '0')}`, 'male');
		}
		await seedMapping('m0', 'MAN 00', 8, 'MALE');

		expect(await syncMonthRosterForClass(CLASS_ID)).toBe(22);

		// 21 male slots shipped with the form, so the 22nd learner needs two more
		// rows spliced in above the MALE TOTAL.
		const rows = await mappings();
		expect(rows).toHaveLength(22);
		expect(rows.at(-1)).toEqual({
			student_id: 'm21',
			row_index: 29,
			gender_block: 'MALE'
		});
		expect(getCellTextAt(await juneSheet(), 'C29')).toBe('MAN 21');
	});
});
