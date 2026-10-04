import { db, useTestDb } from '$lib/db/repos/__tests__/schema';
import { loadTemplate, TEMPLATE_SHEETS } from '$lib/features/excel/__tests__/template-fixture';
import {
	getCellText,
	getCellTextAt,
	openWorkbook,
	writableDayColumns
} from '$lib/features/excel/workbook';
import { errorMessage } from '$lib/db';
import type { AppError } from '$lib/db';
import {
	clearUnusedLearnerMarks,
	expandedRosterSlots,
	rejectDuplicateRosterNames,
	rosterNameMarks,
	studentMappingsFromRosterAssignments,
	syncWorkbookLearnerMappings,
	syncWorkbookLearnerMappingsWithOld,
	syncWorkbookRosterForClass,
	templateOwnsRoster,
	templateRosterAssignments,
	templateRosterSlots,
	uniqueNormalizedName
} from '../roster';
import { readWorkbookAnalysis } from '../roster/analysis';
import { latestTemplateForClass, dateMappingsForTemplate } from '../repository';
import { beforeEach, describe, expect, test } from 'vitest';
import type { Workbook } from 'exceljs';
import type { Sf2WorkbookLearner } from '../calendar';
import type { Sf2TemplateRecord } from '../repository';
import type { Student } from '$lib/domain/models';

useTestDb();

const CLASS_ID = 'class-1';
const MARIA = '11111111-1111-4111-8111-111111111111';
const PEDRO = '22222222-2222-4222-8222-222222222222';

function student(id: string, name: string, gender?: 'male' | 'female'): Student {
	return { id, name, gender, createdAt: '2026-01-01T00:00:00.000Z' };
}

function males(count: number): Student[] {
	return Array.from({ length: count }, (_, index) =>
		student(`m-${index}`, `Male ${index}`, 'male')
	);
}

function females(count: number): Student[] {
	return Array.from({ length: count }, (_, index) =>
		student(`f-${index}`, `Female ${index}`, 'female')
	);
}

function template(sourceHash: string): Sf2TemplateRecord {
	return {
		id: 't1',
		sourcePath: 'C:/workbooks/SF2.xlsx',
		sourceHash,
		schoolId: 'S-1',
		schoolName: 'Test School',
		schoolYear: '2026-2027',
		reportMonth: 'SEPTEMBER',
		gradeLevel: '3',
		section: 'A',
		adviserName: 'Dela Cruz',
		schoolHeadName: 'Santos',
		layoutFingerprint: '',
		activeClassId: CLASS_ID,
		importedAt: 1,
		lastSyncedAt: undefined
	};
}

function learner(rowIndex: number, name: string, sf2LearnerId?: string): Sf2WorkbookLearner {
	return { rowIndex, name, genderBlock: 'MALE', sf2LearnerId };
}

function oldMapping(studentId: string, rowIndex: number, name: string) {
	return {
		templateId: 'old-template',
		studentId,
		workbookName: name,
		normalizedName: name.toUpperCase(),
		rowIndex,
		genderBlock: 'MALE'
	};
}

/** `AppError` is a plain object, not an `Error`, so `toThrow` cannot see it. */
function thrownBy(run: () => unknown): AppError {
	try {
		run();
	} catch (thrown) {
		return thrown as AppError;
	}
	expect.unreachable('the call was expected to fail');
}

async function seedClassWithTwoStudents(): Promise<void> {
	await db().execute(
		`INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
		 VALUES (?, 'Grade 1 - A', '07:00', '13:00', '07:30', 1)`,
		[CLASS_ID]
	);
	for (const [id, name, learnerId] of [
		[MARIA, 'SANTO, MARIA', 'LRN-0001'],
		[PEDRO, 'SANTOS, PEDRO', 'LRN-0002']
	]) {
		await db().execute(
			'INSERT INTO students (id, name, class_id, created_at, sf2_learner_id) VALUES (?, ?, ?, 1, ?)',
			[id, name, CLASS_ID, learnerId]
		);
	}
}

// ── Slot layout ──────────────────────────────────────────────────────────────

describe('templateRosterSlots', () => {
	test('is 21 male rows then 19 female rows, skipping both TOTAL rows', () => {
		const slots = templateRosterSlots();
		expect(slots).toHaveLength(40);
		expect(slots[0]).toEqual({ rowIndex: 8, genderBlock: 'MALE' });
		expect(slots[20]).toEqual({ rowIndex: 28, genderBlock: 'MALE' });
		expect(slots[21]).toEqual({ rowIndex: 30, genderBlock: 'FEMALE' });
		expect(slots[39]).toEqual({ rowIndex: 48, genderBlock: 'FEMALE' });
		expect(slots.some((slot) => slot.rowIndex === 29)).toBe(false);
		expect(slots.some((slot) => slot.rowIndex === 49)).toBe(false);
	});

	test('is all MALE before all FEMALE', () => {
		expect(
			templateRosterSlots()
				.map((slot) => slot.genderBlock)
				.indexOf('FEMALE')
		).toBe(21);
	});
});

describe('expandedRosterSlots', () => {
	test('keeps the female block below the pushed-down MALE TOTAL row', () => {
		const slots = expandedRosterSlots(25, 22);
		expect(slots[0].rowIndex).toBe(8);
		expect(slots[24]).toEqual({ rowIndex: 32, genderBlock: 'MALE' });
		expect(slots[25]).toEqual({ rowIndex: 34, genderBlock: 'FEMALE' });
		expect(slots[46]).toEqual({ rowIndex: 55, genderBlock: 'FEMALE' });
	});
});

// ── Ownership ────────────────────────────────────────────────────────────────

describe('templateOwnsRoster', () => {
	test('is true only for the `bundled-` prefix the app writes', () => {
		expect(templateOwnsRoster(template('bundled-abc123'))).toBe(true);
		expect(templateOwnsRoster(template('bundled-hash'))).toBe(true);
		expect(templateOwnsRoster(template('bundled'))).toBe(false);
		expect(templateOwnsRoster(template('e3b0c44298fc1c149afbf4c8996fb924'))).toBe(false);
		expect(templateOwnsRoster(template('not-bundled-hash'))).toBe(false);
		expect(templateOwnsRoster(template(''))).toBe(false);
	});
});

// ── Assignments ──────────────────────────────────────────────────────────────

describe('templateRosterAssignments', () => {
	test('puts males in the male block and females in the female block', () => {
		const assignments = templateRosterAssignments([
			student('a', 'Juan', 'male'),
			student('b', 'Maria', 'female')
		]);
		expect(assignments.map((entry) => entry.slot)).toEqual([
			{ rowIndex: 8, genderBlock: 'MALE' },
			{ rowIndex: 30, genderBlock: 'FEMALE' }
		]);
	});

	test('refuses a student the school never gave a gender for', () => {
		const error = thrownBy(() => templateRosterAssignments([student('a', 'Unknown')]));
		expect(error.kind).toBe('InvalidInput');
		expect(errorMessage(error)).toMatch(/Set Male\/Female.*Unknown/);
	});

	test('expands past the 21 male slots instead of failing', () => {
		const assignments = templateRosterAssignments(males(22));
		expect(assignments).toHaveLength(22);
		expect(assignments[0].slot.rowIndex).toBe(8);
		expect(assignments[21].slot.rowIndex).toBe(29);
	});

	test('expands the female block without moving the male block', () => {
		const assignments = templateRosterAssignments(females(20));
		expect(assignments[0].slot.rowIndex).toBe(30);
		expect(assignments[19].slot.rowIndex).toBe(49);
	});

	test('expands both blocks when the class outgrows both', () => {
		const assignments = templateRosterAssignments([...males(25), ...females(22)]);
		expect(assignments).toHaveLength(47);
		expect(assignments[24].slot.rowIndex).toBe(32);
		expect(assignments[25].slot.rowIndex).toBe(34);
		expect(assignments[46].slot.rowIndex).toBe(55);
	});

	test('leaves the fixed layout alone below capacity', () => {
		const assignments = templateRosterAssignments([...males(15), ...females(12)]);
		expect(assignments[0].slot.rowIndex).toBe(8);
		expect(assignments[14].slot.rowIndex).toBe(22);
		expect(assignments[15].slot.rowIndex).toBe(30);
		expect(assignments[26].slot.rowIndex).toBe(41);
	});
});

// ── Names ────────────────────────────────────────────────────────────────────

describe('uniqueNormalizedName', () => {
	test('keeps the first spelling of a name', () => {
		const seen = new Set<string>();
		expect(uniqueNormalizedName(seen, 'Juan dela Cruz', '1')).toBe('JUAN DELA CRUZ');
	});

	test('suffixes the second learner of the same name with their id', () => {
		const seen = new Set<string>();
		expect(uniqueNormalizedName(seen, 'Maria Santos', '1')).toBe('MARIA SANTOS');
		expect(uniqueNormalizedName(seen, 'Maria Santos', '2')).toBe('MARIA SANTOS#2');
	});

	test('accumulates a suffix per further collision', () => {
		const seen = new Set<string>();
		uniqueNormalizedName(seen, 'John Smith', '10');
		uniqueNormalizedName(seen, 'John Smith', '20');
		expect(uniqueNormalizedName(seen, 'John Smith', '30')).toBe('JOHN SMITH#30');
	});

	test('leaves unrelated names alone, accented or not', () => {
		const seen = new Set<string>();
		expect(uniqueNormalizedName(seen, 'María José', '1')).toBe('MARÍA JOSÉ');
		expect(uniqueNormalizedName(seen, 'Bob', '2')).toBe('BOB');
	});
});

describe('rejectDuplicateRosterNames', () => {
	test('passes a roster with no duplicates', () => {
		expect(() =>
			rejectDuplicateRosterNames([student('a', 'Juan', 'male'), student('b', 'Maria', 'female')])
		).not.toThrow();
	});

	test('rejects two spellings of one name, case-insensitively', () => {
		const error = thrownBy(() =>
			rejectDuplicateRosterNames([
				student('a', 'Juan dela Cruz', 'male'),
				student('b', 'JUAN DELA CRUZ', 'male')
			])
		);
		expect(errorMessage(error)).toMatch(/Duplicate learner names/);
	});

	test('names every duplicate group', () => {
		const error = thrownBy(() =>
			rejectDuplicateRosterNames([
				student('a', 'Alice', 'female'),
				student('b', 'Alice', 'female'),
				student('c', 'Bob', 'male'),
				student('d', 'Bob', 'male')
			])
		);
		expect(errorMessage(error)).toMatch(/Alice.*Bob/s);
	});

	test('passes an empty roster', () => {
		expect(() => rejectDuplicateRosterNames([])).not.toThrow();
	});
});

// ── Marks and mappings ───────────────────────────────────────────────────────

describe('rosterNameMarks', () => {
	test('writes the item number and the name, restarting the number per gender', () => {
		const assignments = templateRosterAssignments([
			student('a', 'Juan', 'male'),
			student('b', 'Pedro', 'male'),
			student('c', 'Maria', 'female')
		]);
		expect(rosterNameMarks(['JUNE 2025'], assignments)).toEqual([
			{ sheetName: 'JUNE 2025', address: 'A8', value: '1' },
			{ sheetName: 'JUNE 2025', address: 'C8', value: 'Juan' },
			{ sheetName: 'JUNE 2025', address: 'A9', value: '2' },
			{ sheetName: 'JUNE 2025', address: 'C9', value: 'Pedro' },
			// The DepEd form numbers the boys 1..n and the girls 1..m.
			{ sheetName: 'JUNE 2025', address: 'A30', value: '1' },
			{ sheetName: 'JUNE 2025', address: 'C30', value: 'Maria' }
		]);
	});

	test('repeats the same marks for every sheet given', () => {
		const assignments = templateRosterAssignments([student('a', 'Juan', 'male')]);
		expect(rosterNameMarks(['JUNE 2025', 'JULY 2025'], assignments)).toHaveLength(4);
	});
});

describe('studentMappingsFromRosterAssignments', () => {
	test('records the row each student was given', () => {
		const mappings = studentMappingsFromRosterAssignments(
			'template-1',
			templateRosterAssignments([student(MARIA, 'Juan', 'male'), student(PEDRO, 'Maria', 'female')])
		);
		expect(mappings).toEqual([
			{
				templateId: 'template-1',
				studentId: MARIA,
				workbookName: 'Juan',
				normalizedName: 'JUAN',
				rowIndex: 8,
				genderBlock: 'MALE'
			},
			{
				templateId: 'template-1',
				studentId: PEDRO,
				workbookName: 'Maria',
				normalizedName: 'MARIA',
				rowIndex: 30,
				genderBlock: 'FEMALE'
			}
		]);
	});

	test('keeps two learners whose names normalise the same apart', () => {
		const mappings = studentMappingsFromRosterAssignments(
			't1',
			templateRosterAssignments([student('a', 'Juan', 'male'), student('b', 'JUAN', 'male')])
		);
		expect(mappings[0].normalizedName).not.toBe(mappings[1].normalizedName);
	});

	test('is empty for a class with no students', () => {
		expect(studentMappingsFromRosterAssignments('t1', templateRosterAssignments([]))).toEqual([]);
	});
});

// ── Clearing unused learner rows ─────────────────────────────────────────────

describe('clearUnusedLearnerMarks', () => {
	const rowOf = (mark: { address: string }): number => Number(mark.address.replace(/[A-C]/, ''));

	test('clears A, B and C on all 40 fresh slots when nothing is mapped', () => {
		const marks = clearUnusedLearnerMarks(['JANUARY 2025'], []);
		expect(marks).toHaveLength(40 * 3);
		for (const mark of marks) expect(mark.value).toBe('');
		const addresses = new Set(marks.map((mark) => mark.address));
		// The TOTAL rows carry the subtotals, so they are never cleared.
		expect(addresses.has('A29')).toBe(false);
		expect(addresses.has('A49')).toBe(false);
	});

	test('leaves a mapped row untouched', () => {
		const marks = clearUnusedLearnerMarks(['JANUARY 2025'], [8, 9, 30, 31]);
		expect(marks).toHaveLength(36 * 3);
		const cleared = new Set(marks.map(rowOf));
		for (const row of [8, 9, 30, 31]) expect(cleared.has(row)).toBe(false);
	});

	test('writes only to the sheets it is given', () => {
		// 2 visible sheets x 40 slots x 3 columns.
		expect(clearUnusedLearnerMarks(['JANUARY 2025', 'FEBRUARY 2025'], [])).toHaveLength(240);
	});

	test('returns nothing when every slot is mapped', () => {
		const mapped = [
			...Array.from({ length: 21 }, (_, index) => 8 + index),
			...Array.from({ length: 19 }, (_, index) => 30 + index)
		];
		expect(clearUnusedLearnerMarks(['JANUARY 2025'], mapped)).toEqual([]);
	});

	test('uses the grown layout when the workbook was just expanded', () => {
		// 25 male + 22 female puts the female block at 34..55, past the fresh 30..48.
		const rows = new Set(clearUnusedLearnerMarks(['JULY 2026'], [], 25, 22).map(rowOf));
		expect(rows.has(55)).toBe(true);
		expect(rows.has(56)).toBe(false);
	});
});

// ── Learner identity: sf2_learner_id → name → row (spec 6.3 / E7) ─────────────

describe('syncWorkbookLearnerMappings', () => {
	beforeEach(async () => {
		await seedClassWithTwoStudents();
	});

	test('a reshuffled roster matches on the DepEd ID, not the row', async () => {
		// E7: the new file spells Maria differently, so no name matches, and she now
		// sits in the row Pedro used to occupy. Matching on the row would hand her
		// Pedro's identity, and with it Pedro's X marks.
		const sync = await syncWorkbookLearnerMappingsWithOld(
			CLASS_ID,
			'new-template',
			[learner(9, 'SANTO, MARIA L.', 'LRN-0001')],
			[oldMapping(MARIA, 8, 'SANTO, MARIA'), oldMapping(PEDRO, 9, 'SANTOS, PEDRO')]
		);

		expect(sync.studentMappings[0].studentId).toBe(MARIA);
		expect(sync.studentsCreated).toBe(0);
		expect(sync.studentsUpdated).toBe(1);
	});

	test('the workbook is authoritative about how a learner is spelled', async () => {
		await syncWorkbookLearnerMappingsWithOld(
			CLASS_ID,
			'new-template',
			[learner(9, 'SANTO, MARIA L.', 'LRN-0001')],
			[oldMapping(MARIA, 8, 'SANTO, MARIA')]
		);
		const row = await db().queryOne<{ name: string }>('SELECT name FROM students WHERE id = ?', [
			MARIA
		]);
		expect(row?.name).toBe('SANTO, MARIA L.');
	});

	test('a workbook with no DepEd ID still resolves by name', async () => {
		// The bundled template merges the learner-ID cell into the "No." cell, so every
		// learner on it has no ID. That must keep working: the ID is an improvement
		// where the school supplies one, never a requirement.
		const sync = await syncWorkbookLearnerMappingsWithOld(
			CLASS_ID,
			'new-template',
			[learner(14, 'SANTO, MARIA')],
			[oldMapping(MARIA, 8, 'SANTO, MARIA'), oldMapping(PEDRO, 9, 'SANTOS, PEDRO')]
		);
		expect(sync.studentMappings[0].studentId).toBe(MARIA);
		expect(sync.studentsCreated).toBe(0);
	});

	test('creates a learner the app has no record of', async () => {
		const sync = await syncWorkbookLearnerMappings(CLASS_ID, 't', [
			learner(10, 'SANTOS, ANA', 'LRN-0009')
		]);
		expect(sync.studentsCreated).toBe(1);
		const count = await db().queryOne<{ count: number }>('SELECT COUNT(*) AS count FROM students');
		expect(Number(count?.count)).toBe(3);
	});

	test('reads the gender block onto a learner it creates', async () => {
		await syncWorkbookLearnerMappings(CLASS_ID, 't', [
			{ rowIndex: 10, name: 'SANTOS, ANA', genderBlock: 'FEMALE' }
		]);
		const row = await db().queryOne<{ gender: string | null }>(
			'SELECT gender FROM students WHERE name = ?',
			['SANTOS, ANA']
		);
		expect(row?.gender).toBe('female');
	});

	test('reuses a student the class already has rather than duplicating them', async () => {
		const sync = await syncWorkbookLearnerMappings(CLASS_ID, 't', [learner(11, 'SANTOS, PEDRO')]);
		expect(sync.studentsCreated).toBe(0);
		expect(sync.studentsReused).toBe(1);
		expect(sync.studentMappings[0].studentId).toBe(PEDRO);
	});

	test('never lets two workbook rows claim one student', async () => {
		// `sf2_student_mappings` keys on (template_id, student_id), so a second row
		// resolving to the same student would collide on the insert.
		const sync = await syncWorkbookLearnerMappings(CLASS_ID, 't', [
			{ rowIndex: 8, name: 'SANTOS, PEDRO' },
			{ rowIndex: 9, name: 'SANTOS, PEDRO' }
		]);
		expect(sync.studentMappings).toHaveLength(1);
	});

	test('ignores the form’s own rows in the name column', async () => {
		const sync = await syncWorkbookLearnerMappings(CLASS_ID, 't', [
			{ rowIndex: 5, name: 'NAME (LAST NAME, FIRST NAME, MIDDLE NAME)', genderBlock: 'MALE' },
			{ rowIndex: 29, name: 'MALE TOTAL', genderBlock: 'MALE' }
		]);
		expect(sync.studentMappings).toEqual([]);
	});
});

// ── Reading the real bundled template ────────────────────────────────────────

describe('readWorkbookAnalysis', () => {
	test('reads the bundled template without needing Excel installed', async () => {
		const fixture = await loadTemplate();
		const analysis = readWorkbookAnalysis(await fixture.open());

		expect(analysis.sheets.map((sheet) => sheet.name)).toEqual(TEMPLATE_SHEETS);
		expect(analysis.learners.length).toBeGreaterThan(0);
		for (const learner of analysis.learners) {
			expect(learner.genderBlock).toBeDefined();
			expect(learner.rowIndex).toBeGreaterThanOrEqual(8);
			expect(learner.rowIndex).toBeLessThanOrEqual(48);
		}
		expect(analysis.learners.some((learner) => learner.genderBlock === 'MALE')).toBe(true);
		expect(analysis.learners.some((learner) => learner.genderBlock === 'FEMALE')).toBe(true);
	});

	test('maps every writable day column once, never the merged half', async () => {
		const fixture = await loadTemplate();
		const workbook = await fixture.open();
		const analysis = readWorkbookAnalysis(workbook);

		const june = analysis.dates.filter((date) => date.sheetName === 'JUNE 2025');
		expect(june.length).toBeGreaterThan(0);
		// A merged pair answers for its master, so counting both halves would give
		// every day twice - two writes fighting over one cell.
		expect(new Set(june.map((date) => date.columnLetter)).size).toBe(june.length);
		const writable = writableDayColumns(workbook.getWorksheet('JUNE 2025')!);
		for (const date of june) {
			expect(writable).toContain(date.columnLetter);
			expect(date.date.startsWith('2025-06-')).toBe(true);
		}
	});

	test('reads no date out of `COMPLETE DAYS`, which names no month', async () => {
		const fixture = await loadTemplate();
		const analysis = readWorkbookAnalysis(await fixture.open());
		expect(analysis.dates.some((date) => date.sheetName === 'COMPLETE DAYS')).toBe(false);
	});

	test('the bundled template carries no DepEd ID, so no learner claims one', async () => {
		// The bundled template merges A8:B8, so column 2 is the item number. Accepting
		// it would give every month the same positional identity.
		const fixture = await loadTemplate();
		const analysis = readWorkbookAnalysis(await fixture.open());
		expect(analysis.learners.every((learner) => learner.sf2LearnerId === undefined)).toBe(true);
	});

	test('reads the header block off the first monthly sheet', async () => {
		const fixture = await loadTemplate();
		const analysis = readWorkbookAnalysis(await fixture.open());
		expect(analysis.gradeLevel).not.toBe('');
		expect(analysis.reportMonth).not.toBe('');
		expect(analysis.adviserName).not.toBe('');
	});
});
// -- The roster sync, end to end on the real bundled template -----------------

/**
 * The pre-split tables, recreated from the real migration shape. `repository.test.ts`
 * carries the same DDL for the same reason: the shared repo fixture declares
 * `sf2_student_mappings.id` as a primary key the real table does not have.
 */
const SF2_SCHEMA = `
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

DROP TABLE IF EXISTS sf2_date_mappings;
CREATE TABLE sf2_date_mappings (
	template_id TEXT NOT NULL,
	sheet_name TEXT,
	date TEXT NOT NULL,
	column_letter TEXT NOT NULL,
	column_index INTEGER NOT NULL,
	PRIMARY KEY (template_id, date)
);
`;

const WORKBOOK_PATH = '/workbooks/SF2-GRADE-3-MATAPAT.xlsx';

async function installTemplate(sourceHash = 'bundled-abc123-class-1'): Promise<void> {
	await db().script(SF2_SCHEMA);
	await db().execute(
		`INSERT OR IGNORE INTO classes (id, name, day_start, day_end, late_after, created_at)
		 VALUES (?, 'Grade 3 - MATAPAT', '08:00', '15:00', '08:45', 1)`,
		[CLASS_ID]
	);
	await db().execute('DELETE FROM sf2_templates');
	await db().execute(
		`INSERT INTO sf2_templates
		 (id, source_path, source_hash, grade_level, section, active_class_id, imported_at)
		 VALUES ('t1', ?, ?, '3', 'MATAPAT', ?, 1)`,
		[WORKBOOK_PATH, sourceHash, CLASS_ID]
	);
}

async function addStudent(id: string, name: string, gender: 'male' | 'female'): Promise<void> {
	await db().execute(
		'INSERT INTO students (id, name, gender, class_id, created_at) VALUES (?, ?, ?, ?, 1)',
		[id, name, gender, CLASS_ID]
	);
}

/**
 * The first day column the workbook actually mapped for a sheet.
 *
 * Not the first writable column: some writable columns hold a stale day number from
 * another month, and a mapping is only a column the analysis resolved to a real date.
 */
function firstMappedDay(workbook: Workbook, sheetName: string): string {
	const first = readWorkbookAnalysis(workbook).dates.find((date) => date.sheetName === sheetName);
	if (first === undefined) throw new Error(`the workbook mapped no day for ${sheetName}`);
	return first.columnLetter;
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

describe('syncWorkbookRosterForClass', () => {
	beforeEach(async () => {
		const fixture = await loadTemplate();
		await fixture.fileSystem.writeFileAtomic(
			WORKBOOK_PATH,
			await fixture.fileSystem.readFile(fixture.path)
		);
		await installTemplate();
	});

	test('a class with no workbook is not an error', async () => {
		await syncWorkbookRosterForClass('class-with-no-workbook');
		expect(await latestTemplateForClass('class-with-no-workbook')).toBeUndefined();
	});

	test('refuses a roster that holds the same learner twice', async () => {
		await addStudent('s1', 'JUAN DELA CRUZ', 'male');
		await addStudent('s2', 'juan dela cruz', 'male');
		expect(await failure(syncWorkbookRosterForClass(CLASS_ID))).toMatch(/Duplicate learner names/);
	});

	test('refuses a class the school gave no gender for', async () => {
		await db().execute(
			"INSERT INTO students (id, name, class_id, created_at) VALUES ('s1', 'JUAN', ?, 1)",
			[CLASS_ID]
		);
		expect(await failure(syncWorkbookRosterForClass(CLASS_ID))).toMatch(/Set Male\/Female.*JUAN/);
	});

	test('refuses when the workbook file has gone', async () => {
		await addStudent('s1', 'DELA CRUZ, JUAN', 'male');
		await db().execute('UPDATE sf2_templates SET source_path = ? WHERE id = ?', [
			'/workbooks/gone.xlsx',
			't1'
		]);
		expect(await failure(syncWorkbookRosterForClass(CLASS_ID))).toMatch(
			/The app SF2 working workbook no longer exists/
		);
	});

	describe('a bundled working copy', () => {
		beforeEach(async () => {
			await addStudent('s1', 'DELA CRUZ, JUAN', 'male');
			await addStudent('s2', 'SANTOS, MARIA', 'female');
		});

		test('gives every student a row and records the mapping', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);

			const rows = await db().query<{
				student_id: string;
				row_index: number;
				gender_block: string;
			}>('SELECT student_id, row_index, gender_block FROM sf2_student_mappings ORDER BY row_index');
			expect(rows).toEqual([
				{ student_id: 's1', row_index: 8, gender_block: 'MALE' },
				{ student_id: 's2', row_index: 30, gender_block: 'FEMALE' }
			]);
		});

		test('writes the names and the item numbers onto every monthly sheet', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);
			const workbook = await openWorkbook(WORKBOOK_PATH);

			for (const sheet of workbook.worksheets.filter(
				(s) => s.state === 'visible' && s.name !== 'COMPLETE DAYS'
			)) {
				expect(getCellText(sheet, 8, 3).trim()).toBe('DELA CRUZ, JUAN');
				expect(getCellText(sheet, 30, 3).trim()).toBe('SANTOS, MARIA');
				expect(getCellText(sheet, 8, 1)).toBe('1');
				expect(getCellText(sheet, 30, 1)).toBe('1');
			}
		});

		test('empties the learner rows nothing sits in, so the sample class is gone', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);
			const sheet = (await openWorkbook(WORKBOOK_PATH)).getWorksheet('JUNE 2025')!;
			expect(getCellText(sheet, 9, 3).trim()).toBe('');
			expect(getCellText(sheet, 31, 3).trim()).toBe('');
		});

		test('hides the empty rows and leaves the TOTAL rows visible', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);
			const sheet = (await openWorkbook(WORKBOOK_PATH)).getWorksheet('JUNE 2025')!;
			expect(sheet.getRow(8).hidden).toBe(false);
			expect(sheet.getRow(30).hidden).toBe(false);
			expect(sheet.getRow(9).hidden).toBe(true);
			expect(sheet.getRow(29).hidden).toBe(false);
		});

		test('rewrites the TOTAL Per Day formulas, with their cached values', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);
			const workbook = await openWorkbook(WORKBOOK_PATH);
			const sheet = workbook.getWorksheet('JUNE 2025')!;
			const firstDay = firstMappedDay(workbook, 'JUNE 2025');
			// 1 boy, 1 girl, nobody absent yet.
			expect(getCellTextAt(sheet, `${firstDay}29`)).toBe('1');
			expect(getCellTextAt(sheet, `${firstDay}49`)).toBe('1');
			expect(getCellTextAt(sheet, `${firstDay}50`)).toBe('2');
		});

		test('records the day grid and refreshes the layout fingerprint', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);
			expect((await dateMappingsForTemplate('t1')).length).toBeGreaterThan(0);
			expect((await latestTemplateForClass(CLASS_ID))?.layoutFingerprint).not.toBe('');
		});

		test('a second sync is idempotent', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);
			const first = await db().query('SELECT * FROM sf2_student_mappings ORDER BY row_index');
			await syncWorkbookRosterForClass(CLASS_ID);
			const second = await db().query('SELECT * FROM sf2_student_mappings ORDER BY row_index');
			expect(second).toEqual(first);
		});
	});

	describe('a workbook the school handed over', () => {
		beforeEach(async () => {
			await installTemplate('school-hash');
			await addStudent('s1', 'DELA CRUZ, JUAN', 'male');
			await addStudent('s2', 'SANTOS, MARIA', 'female');
		});

		test('gives each new student a row the school left free, writing column C only', async () => {
			const before = (await openWorkbook(WORKBOOK_PATH)).getWorksheet('JUNE 2025')!;
			const itemNumberBefore = getCellText(before, 8, 1);

			await syncWorkbookRosterForClass(CLASS_ID);

			const rows = await db().query<{
				student_id: string;
				row_index: number;
				gender_block: string;
			}>('SELECT student_id, row_index, gender_block FROM sf2_student_mappings ORDER BY row_index');
			expect(rows).toEqual([
				{ student_id: 's1', row_index: 8, gender_block: 'MALE' },
				{ student_id: 's2', row_index: 30, gender_block: 'FEMALE' }
			]);

			const sheet = (await openWorkbook(WORKBOOK_PATH)).getWorksheet('JUNE 2025')!;
			expect(getCellText(sheet, 8, 3).trim()).toBe('DELA CRUZ, JUAN');
			// The school's own `No.` column is the school's: renumbering it would move
			// every name below it.
			expect(getCellText(sheet, 8, 1)).toBe(itemNumberBefore);
		});

		test('leaves the workbook and the template alone once every student sits in a row', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);
			const after = await db().query('SELECT * FROM sf2_student_mappings ORDER BY row_index');
			const fingerprint = (await latestTemplateForClass(CLASS_ID))?.layoutFingerprint;

			await syncWorkbookRosterForClass(CLASS_ID);

			expect(await db().query('SELECT * FROM sf2_student_mappings ORDER BY row_index')).toEqual(
				after
			);
			expect((await latestTemplateForClass(CLASS_ID))?.layoutFingerprint).toBe(fingerprint);
		});

		test('refuses a class that has outgrown the rows the school left free', async () => {
			await syncWorkbookRosterForClass(CLASS_ID);
			// Far more boys than the template's 21 male slots can hold.
			for (let index = 0; index < 30; index += 1) {
				await addStudent(`x-${index}`, `EXTRA BOY ${index}`, 'male');
			}
			const rows = await db().query<{ row_index: number; gender_block: string | null }>(
				'SELECT row_index, gender_block FROM sf2_student_mappings ORDER BY row_index'
			);
			expect(await failure(syncWorkbookRosterForClass(CLASS_ID))).toMatch(
				/learner rows in total.*add rows for the extra learners/s
			);
			// Nothing was written: a refusal must not leave half a class in the file.
			expect(
				await db().query(
					'SELECT row_index, gender_block FROM sf2_student_mappings ORDER BY row_index'
				)
			).toEqual(rows);
			const count = await db().queryOne<{ count: number }>(
				'SELECT COUNT(*) AS count FROM sf2_student_mappings'
			);
			expect(Number(count?.count)).toBe(2);
		});
	});
});
