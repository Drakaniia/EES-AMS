/**
 * SF2 open parity — the open path matches the old Rust behavior, adapted from
 * COM/`.xls` to ExcelJS/`.xlsx` (spec `docs/specs/sf2-open-parity-spec.md`).
 *
 * Ground truth is the old output's layout, which the bundled
 * `TEMPLATE_AUTOMATED_SF2.xlsx` preserves (Excel's own `.xls`→`.xlsx`
 * conversion): 25 labelled day cells across F–AL, merged weekday pairs, the
 * eight header fields, and the formula text of `attendance_marks.rs`.
 *
 * Real template bytes plus a real (in-memory) database throughout: no fixture
 * invents sheet names, merges or cell addresses.
 */
import { describe, expect, it } from 'vitest';
import { createStudent } from '$lib/db/repos/students';
import {
	formulaOf,
	formulaResult,
	getCellTextAt,
	hasFormula,
	openWorkbook
} from '$lib/features/excel/workbook';
import { loadTemplate } from '$lib/features/excel/__tests__/template-fixture';
import { db, insertMonthTemplate, seedClass, useAttendanceTestDb } from './attendance-fixture';
import { setAttendanceEventForDay } from '../attendance/attendance-events';
import { syncAndOpenSf2Workbook, WORKBOOK_MISSING_MESSAGE } from '../attendance/attendance-service';
import { resolveMonthWriteContext } from '../attendance/write-context';
import {
	actionFor,
	decide,
	guardBeforeWrite,
	staleAbortMessage,
	NO_MAPPED_DATES,
	NO_MAPPED_LEARNERS,
	WORKBOOK_NOT_READABLE,
	type SyncPermit
} from '../guard';
import type { Sf2GridCell } from '../attendance/attendance-marks';
import type { Sf2ProgressUpdate } from '../progress';

useAttendanceTestDb();

const JUNE = 'JUNE';
const JUNE_SCHOOL_YEAR = '2025-2026';
const JUNE_SHEET = 'JUNE 2025';

/** June 2025 day columns on the real template: Mon 2 → F, Tue 3 → H. */
const JUNE_DAYS = [
	{ date: '2025-06-02', columnLetter: 'F', columnIndex: 6 },
	{ date: '2025-06-03', columnLetter: 'H', columnIndex: 8 }
];

/** The template's own weekday header: 25 labels across F–AL. */
const WEEKDAY_LABELS: [string, string][] = [
	['F', 'M'],
	['H', 'T'],
	['I', 'W'],
	['J', 'TH'],
	['K', 'F'],
	['L', 'M'],
	['N', 'T'],
	['O', 'W'],
	['P', 'TH'],
	['Q', 'F'],
	['R', 'M'],
	['T', 'T'],
	['U', 'W'],
	['V', 'TH'],
	['X', 'F'],
	['Z', 'M'],
	['AB', 'T'],
	['AC', 'W'],
	['AD', 'TH'],
	['AE', 'F'],
	['AF', 'M'],
	['AG', 'T'],
	['AI', 'W'],
	['AJ', 'TH'],
	['AK', 'F']
];

function cell(sheetName: string, columnLetter: string, rowIndex: number): Sf2GridCell {
	return { sheetName, columnLetter, rowIndex };
}

async function seedJuneMonth(
	classId: string,
	path: string,
	students: { id: string; block: string }[]
) {
	await insertMonthTemplate({
		classId,
		schoolYear: JUNE_SCHOOL_YEAR,
		reportMonth: JUNE,
		reportYear: 2025,
		sourcePath: path,
		firstSchoolDay: 2
	});
	const stored = await db().queryOne<{ id: string }>(
		'SELECT id FROM sf2_month_templates WHERE active_class_id = ?',
		[classId]
	);
	const templateId = stored?.id ?? '';
	const rows = [8, 30];
	let index = 0;
	for (const student of students) {
		const rowIndex = rows[index++] ?? 8;
		const record = await db().queryOne<{ name: string }>('SELECT name FROM students WHERE id = ?', [
			student.id
		]);
		const name = record?.name ?? `LEARNER ${rowIndex}`;
		await db().execute(
			`INSERT INTO sf2_month_student_mappings
			   (template_id, student_id, workbook_name, normalized_name, row_index, gender_block)
			 VALUES (?, ?, ?, ?, ?, ?)`,
			[templateId, student.id, name, name.toUpperCase(), rowIndex, student.block]
		);
	}
	for (const day of JUNE_DAYS) {
		await db().execute(
			`INSERT INTO sf2_month_date_mappings
			   (template_id, date, column_letter, column_index, sheet_name)
			 VALUES (?, ?, ?, ?, ?)`,
			[templateId, day.date, day.columnLetter, day.columnIndex, JUNE_SHEET]
		);
	}
}

async function seedTwoStudents(classId: string) {
	const first = await createStudent({ classId, name: 'Dela Cruz, Juan' });
	const second = await createStudent({ classId, name: 'Santos, Maria' });
	return { firstId: first.id, secondId: second.id };
}

async function recordAbsent(studentId: string, classId: string, date: string) {
	await setAttendanceEventForDay({
		studentId,
		classId,
		date,
		dayStart: '07:30',
		eventType: 'absent',
		reason: 'parity-test'
	});
}

describe('guard decide/action', () => {
	const labels = new Map<string, [string, string]>([['JUNE 2025!F8', ['LEARNER 8', '2025-06-02']]]);

	it('proves only when the database holds every workbook mark', () => {
		const dbCells = [cell('JUNE 2025', 'F', 8), cell('JUNE 2025', 'H', 8)];
		expect(decide(dbCells, [cell('JUNE 2025', 'F', 8)], labels, 'reason')).toMatchObject({
			kind: 'Proven',
			dbCount: 2,
			workbookCount: 1
		});
		// Right count, wrong day: still Stale.
		const wrongDay = decide(dbCells, [cell('JUNE 2025', 'I', 8)], labels, 'reason');
		expect(wrongDay.kind).toBe('Stale');
		if (wrongDay.kind === 'Stale') expect(wrongDay.missing).toHaveLength(1);
	});

	it('is Unmeasured when the workbook could not be read', () => {
		expect(decide([], undefined, labels, WORKBOOK_NOT_READABLE)).toEqual({
			kind: 'Unmeasured',
			reason: WORKBOOK_NOT_READABLE
		});
	});

	it('maps permits to actions, and only Rewrite permits a write', () => {
		expect(actionFor({ kind: 'Proven', dbCount: 1, workbookCount: 1 }).kind).toBe('Rewrite');
		expect(actionFor({ kind: 'Unmeasured', reason: NO_MAPPED_DATES }).kind).toBe('ReadOnly');
		expect(
			actionFor({
				kind: 'Stale',
				dbCount: 0,
				workbookCount: 1,
				missing: [['s', 'd']]
			}).kind
		).toBe('ImportThenRecheck');
	});

	it('imports once and rewrites when the import recovers the marks', async () => {
		let imported: [string, string][] = [];
		let calls = 0;
		const stale: SyncPermit = {
			kind: 'Stale',
			dbCount: 0,
			workbookCount: 1,
			missing: [['LEARNER 8', '2025-06-02']]
		};
		const action = await guardBeforeWrite(
			() => {
				calls += 1;
				return calls === 1 ? stale : { kind: 'Proven', dbCount: 1, workbookCount: 1 };
			},
			async (missing) => {
				imported = missing;
			}
		);
		expect(action.kind).toBe('Rewrite');
		expect(imported).toEqual([['LEARNER 8', '2025-06-02']]);
	});

	it('aborts with the stale message when the workbook is still ahead', async () => {
		const stale: SyncPermit = {
			kind: 'Stale',
			dbCount: 0,
			workbookCount: 2,
			missing: [
				['a', 'b'],
				['c', 'd']
			]
		};
		const action = await guardBeforeWrite(
			() => stale,
			() => {}
		);
		expect(action).toEqual({ kind: 'Aborted', message: staleAbortMessage(2) });
		expect(staleAbortMessage(2)).toContain('2 X marks');
	});

	it('passes ReadOnly through with zero writes', async () => {
		let imported = false;
		const action = await guardBeforeWrite(
			() => ({ kind: 'Unmeasured', reason: NO_MAPPED_LEARNERS }),
			() => {
				imported = true;
			}
		);
		expect(action).toEqual({ kind: 'ReadOnly', reason: NO_MAPPED_LEARNERS });
		expect(imported).toBe(false);
	});
});

describe('hybrid resolution', () => {
	it('derives the month from the latest row when the asked month has none', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [{ id: 's1', block: 'MALE' }]);
		const context = await resolveMonthWriteContext(classId, 'MARCH', JUNE_SCHOOL_YEAR);
		expect(context.reportMonth).toBe(JUNE);
		expect(context.reportYear).toBe(2025);
		expect(context.sheetName).toBe(JUNE_SHEET);
	});

	it('refuses with the Rust wording when no row exists at all', async () => {
		const { classId } = await seedClass();
		await expect(resolveMonthWriteContext(classId, JUNE, JUNE_SCHOOL_YEAR)).rejects.toMatchObject({
			detail: 'No SF2 template imported for this class'
		});
	});
});

describe('open writes the calendar', () => {
	it('writes every June date through the merge master and keeps all 25 labels', async () => {
		const { classId } = await seedClass();
		const { firstId: learner, secondId: other } = await seedTwoStudents(classId);
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [
			{ id: learner, block: 'MALE' },
			{ id: other, block: 'FEMALE' }
		]);

		await syncAndOpenSf2Workbook({ classId, reportMonth: JUNE });

		const workbook = await openWorkbook(fixture.path);
		const sheet = workbook.getWorksheet(JUNE_SHEET);
		expect(sheet).toBeDefined();
		// Monday the 2nd on F, Tuesday the 3rd on H — the template's own columns.
		expect(getCellTextAt(sheet!, 'F6')).toBe('2');
		expect(getCellTextAt(sheet!, 'H6')).toBe('3');
		// Empty slots are cleared, not left holding another month's numbers.
		expect(getCellTextAt(sheet!, 'AG6')).toBe('');
		// All 25 weekday labels untouched by the date write (master-only writes).
		for (const [column, label] of WEEKDAY_LABELS) {
			expect(getCellTextAt(sheet!, `${column}7`)).toBe(label);
		}
	});

	it('refuses a weekend first school day instead of silently falling back', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [{ id: 's1', block: 'MALE' }]);
		// 2025-06-01 is a Sunday.
		await db().execute(
			'UPDATE sf2_month_templates SET first_school_day = 1 WHERE active_class_id = ?',
			[classId]
		);
		await expect(syncAndOpenSf2Workbook({ classId, reportMonth: JUNE })).rejects.toMatchObject({
			detail: 'First attendance day must be a Monday-Friday school day'
		});
	});

	it('refuses an out-of-range first school day', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [{ id: 's1', block: 'MALE' }]);
		// June has 30 days.
		await db().execute(
			'UPDATE sf2_month_templates SET first_school_day = 31 WHERE active_class_id = ?',
			[classId]
		);
		await expect(syncAndOpenSf2Workbook({ classId, reportMonth: JUNE })).rejects.toMatchObject({
			detail: 'First attendance day must be between 1 and 30 for this report month'
		});
	});
});

describe('open writes marks, roster, header and formulas', () => {
	it('marks the mapped cells, names the roster, stamps the header and lands on the month tab', async () => {
		const { classId } = await seedClass();
		const { firstId, secondId } = await seedTwoStudents(classId);
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [
			{ id: firstId, block: 'MALE' },
			{ id: secondId, block: 'FEMALE' }
		]);
		await db().execute(
			`UPDATE sf2_month_templates
			 SET school_id = '100200', school_name = 'Test Elementary',
			     grade_level = 'Grade 3', section = 'Matapat',
			     adviser_name = 'Adviser', school_head_name = 'Head'
			 WHERE active_class_id = ?`,
			[classId]
		);
		await recordAbsent(firstId, classId, '2025-06-02');

		const path = await syncAndOpenSf2Workbook({ classId, reportMonth: JUNE });
		expect(path).toBe(fixture.path);

		const workbook = await openWorkbook(fixture.path);
		const sheet = workbook.getWorksheet(JUNE_SHEET)!;
		// X on the mapped learner row and day column only.
		expect(getCellTextAt(sheet, 'F8')).toBe('X');
		expect(getCellTextAt(sheet, 'F30')).toBe('');
		expect(getCellTextAt(sheet, 'H8')).toBe('');
		// Roster names and item numbers on the mapped rows.
		expect(getCellTextAt(sheet, 'C8')).toContain('Dela Cruz');
		expect(getCellTextAt(sheet, 'A8')).toBe('1');
		expect(getCellTextAt(sheet, 'C30')).toContain('Santos');
		// The eight header fields.
		expect(getCellTextAt(sheet, 'F3')).toBe('100200');
		expect(getCellTextAt(sheet, 'F4')).toBe('Test Elementary');
		expect(getCellTextAt(sheet, 'AA4')).toBe('Grade 3');
		// Learner formulas carry the value Excel would cache.
		const am8 = sheet.getCell('AM8');
		expect(hasFormula(am8)).toBe(true);
		expect(formulaOf(am8)).toBe('COUNTIF(F8:AL8,"X")');
		expect(formulaResult(am8)).toBe(1);
		const ao8 = sheet.getCell('AO8');
		expect(hasFormula(ao8)).toBe(true);
		expect(formulaOf(ao8)).toBe('$AW$5-AM8');
		expect(formulaResult(ao8)).toBe(1);
		expect(sheet.getCell('AW5').value).toBe(2);
		// The synced month is the landing tab.
		const index = workbook.worksheets.findIndex((entry) => entry.name === JUNE_SHEET);
		expect(workbook.views[0]?.activeTab).toBe(index);
	});

	it('leaves marks outside the mapped scope alone', async () => {
		const { classId } = await seedClass();
		const { firstId, secondId } = await seedTwoStudents(classId);
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [
			{ id: firstId, block: 'MALE' },
			{ id: secondId, block: 'FEMALE' }
		]);
		// An X outside the mapped day columns: no mapping names it, no sync clears it.
		const planted = await fixture.open();
		planted.getWorksheet(JUNE_SHEET)!.getCell('AL8').value = 'X';
		await fixture.save(planted);

		await syncAndOpenSf2Workbook({ classId, reportMonth: JUNE });

		const workbook = await openWorkbook(fixture.path);
		expect(getCellTextAt(workbook.getWorksheet(JUNE_SHEET)!, 'AL8')).toBe('X');
	});
});

describe('open guard behavior', () => {
	it('imports the workbook marks the database is missing, then rewrites', async () => {
		const { classId } = await seedClass();
		const { firstId, secondId } = await seedTwoStudents(classId);
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [
			{ id: firstId, block: 'MALE' },
			{ id: secondId, block: 'FEMALE' }
		]);
		const planted = await fixture.open();
		planted.getWorksheet(JUNE_SHEET)!.getCell('F8').value = 'X';
		await fixture.save(planted);

		await syncAndOpenSf2Workbook({ classId, reportMonth: JUNE });

		const events = await db().query<{ student_id: string }>(
			`SELECT student_id FROM events WHERE event_type = 'absent'`
		);
		expect(events.some((row) => row.student_id === firstId)).toBe(true);
		const workbook = await openWorkbook(fixture.path);
		expect(getCellTextAt(workbook.getWorksheet(JUNE_SHEET)!, 'F8')).toBe('X');
	});

	it('opens read-only with zero writes when the workbook cannot be read', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [
			{ id: 's1', block: 'MALE' },
			{ id: 's2', block: 'FEMALE' }
		]);
		await fixture.fileSystem.writeFileAtomic(fixture.path, new Uint8Array([1, 2, 3]));

		const messages: string[] = [];
		const path = await syncAndOpenSf2Workbook({
			classId,
			reportMonth: JUNE,
			progress: (update: Sf2ProgressUpdate) => messages.push(update.message)
		});

		expect(path).toBe(fixture.path);
		expect(messages.some((message) => message.startsWith('Opening read-only:'))).toBe(true);
		// Zero writes: the bytes are untouched and no sync was stamped.
		expect(await fixture.fileSystem.readFile(fixture.path)).toEqual(new Uint8Array([1, 2, 3]));
		const stored = await db().queryOne<{ last_synced_at: number | null }>(
			'SELECT last_synced_at FROM sf2_month_templates WHERE active_class_id = ?',
			[classId]
		);
		expect(stored?.last_synced_at ?? null).toBeNull();
	});

	it('reports the ten steps including the guard check', async () => {
		const { classId } = await seedClass();
		const { firstId, secondId } = await seedTwoStudents(classId);
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [
			{ id: firstId, block: 'MALE' },
			{ id: secondId, block: 'FEMALE' }
		]);

		const updates: Sf2ProgressUpdate[] = [];
		await syncAndOpenSf2Workbook({
			classId,
			reportMonth: JUNE,
			progress: (update: Sf2ProgressUpdate) => updates.push(update)
		});

		const messages = updates.map((update) => update.message);
		expect(messages).toContain('Checking the workbook against the app…');
		expect(messages[messages.length - 1]).toBe('Done!');
		const outer = updates.filter((update) => update.total === 10).map((update) => update.current);
		expect(outer[0]).toBe(1);
		expect(outer[outer.length - 1]).toBe(10);
	});

	it('keeps the missing-file refusal', async () => {
		const { classId } = await seedClass();
		const fixture = await loadTemplate();
		await seedJuneMonth(classId, fixture.path, [{ id: 's1', block: 'MALE' }]);
		await db().execute('UPDATE sf2_month_templates SET source_path = ? WHERE active_class_id = ?', [
			'/workbooks/missing.xlsx',
			classId
		]);
		await expect(syncAndOpenSf2Workbook({ classId, reportMonth: JUNE })).rejects.toMatchObject({
			detail: WORKBOOK_MISSING_MESSAGE
		});
	});
});
