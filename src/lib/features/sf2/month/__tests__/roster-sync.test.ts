import { describe, expect, test } from 'vitest';
import ExcelJS from 'exceljs';
import { db, insertMonthTemplate, useMonthTestDb } from './schema';
import { loadTemplate, WORKBOOK_PATH } from '$lib/features/excel/__tests__/template-fixture';
import { getCellTextAt, openWorkbook } from '$lib/features/excel/workbook';
import { totalsOn } from '$lib/features/sf2/attendance/attendance-write';
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

		// 21 male slots shipped with the form, so the 22nd learner needs one
		// more row spliced in above the MALE TOTAL.
		const rows = await mappings();
		expect(rows).toHaveLength(22);
		expect(rows.at(-1)).toEqual({
			student_id: 'm21',
			row_index: 29,
			gender_block: 'MALE'
		});
		expect(getCellTextAt(await juneSheet(), 'C29')).toBe('MAN 21');
	});

	test('a class that still fits its slots is not grown when a learner is added', async () => {
		// 15 boys on rows 8-22 and 10 girls on rows 30-39, then one more boy:
		// six free male slots remain, so no row may move and no merge may go.
		await seedJuneMonth();
		for (let index = 0; index < 15; index += 1) {
			const row = 8 + index;
			await addStudent(`m${index}`, `MAN ${String(index).padStart(2, '0')}`, 'male');
			await seedMapping(`m${index}`, `MAN ${String(index).padStart(2, '0')}`, row, 'MALE');
		}
		for (let index = 0; index < 10; index += 1) {
			const row = 30 + index;
			await addStudent(`f${index}`, `LASS ${String(index).padStart(2, '0')}`, 'female');
			await seedMapping(`f${index}`, `LASS ${String(index).padStart(2, '0')}`, row, 'FEMALE');
		}
		await addStudent('new', 'YBANEZ, ALISTAIR M', 'male');

		expect(await syncMonthRosterForClass(CLASS_ID)).toBe(26);

		const rows = await mappings();
		expect(rows.find((row) => row.student_id === 'new')).toEqual({
			student_id: 'new',
			row_index: 23,
			gender_block: 'MALE'
		});

		const sheet = await juneSheet();
		// The TOTAL rows never moved…
		const totals = totalsOn(sheet);
		expect(totals.maleTotalRow).toBe(29);
		expect(totals.femaleTotalRow).toBe(49);
		expect(totals.combinedTotalRow).toBe(50);
		expect(getCellTextAt(sheet, 'C23')).toBe('YBANEZ, ALISTAIR M');
		expect(getCellTextAt(sheet, 'C29')).toContain('MALE');
		expect(getCellTextAt(sheet, 'C30')).toBe('LASS 00');
		// …and the form is still one merged form: every merge survived and the
		// sampled slaves are still merge slaves, not smeared literal copies.
		expect(sheet.model.merges.length).toBe(681);
		for (const address of ['G8', 'D30', 'AG56']) {
			expect(sheet.getCell(address).type, address).toBe(ExcelJS.ValueType.Merge);
		}
	});

	test('a genuine growth keeps every merge and styles the new rows like learner rows', async () => {
		await seedJuneMonth();
		for (let index = 0; index < 22; index += 1) {
			await addStudent(`m${index}`, `MAN ${String(index).padStart(2, '0')}`, 'male');
		}
		await seedMapping('m0', 'MAN 00', 8, 'MALE');

		expect(await syncMonthRosterForClass(CLASS_ID)).toBe(22);

		const sheet = await juneSheet();
		const totals = totalsOn(sheet);
		expect(totals.maleTotalRow).toBe(30);
		expect(totals.femaleTotalRow).toBe(50);
		expect(totals.combinedTotalRow).toBe(51);
		expect(getCellTextAt(sheet, 'C30')).toContain('MALE');
		expect(sheet.model.merges.length).toBe(681);
		// The spliced-in row draws like the learner row above it.
		expect(sheet.getRow(29).height).toBe(19.5);
		expect(sheet.getCell('F29').border.left?.style).toBe('medium');
	});
});
