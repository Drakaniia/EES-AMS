import { beforeEach, describe, expect, test } from 'vitest';
import {
	depedLearnerIdFromCells,
	deleteMonthRoster,
	matchRosterLearner,
	monthMappingForNormalizedName,
	monthMappingsForLearnerId,
	monthRosterForTemplate,
	normalizeLearnerName,
	replaceMonthRoster,
	setStudentLearnerIds,
	type Sf2MonthStudentMapping
} from '../students';
import { db, insertMonthTemplate, useMonthTestDb } from './schema';

useMonthTestDb();

function roster(over: Partial<Sf2MonthStudentMapping> = {}): Sf2MonthStudentMapping {
	return {
		templateId: 'month-1',
		studentId: 'stu-1',
		workbookName: 'DELA CRUZ, JUAN',
		normalizedName: 'DELA CRUZ,JUAN',
		rowIndex: 8,
		genderBlock: 'MALE',
		...over
	};
}

describe('normalizeLearnerName', () => {
	test('collapses whitespace, closes the comma and uppercases', () => {
		expect(normalizeLearnerName('dela  cruz,  juan')).toBe('DELA CRUZ,JUAN');
		expect(normalizeLearnerName('  DELA CRUZ, JUAN  ')).toBe('DELA CRUZ,JUAN');
	});
});

describe('depedLearnerIdFromCells', () => {
	test('keeps a real DepEd ID', () => {
		expect(depedLearnerIdFromCells('13672845021', '1')).toBe('13672845021');
		expect(depedLearnerIdFromCells('LRN-0001', '7')).toBe('LRN-0001');
	});

	test('rejects the item number the merged No. cell reads back', () => {
		// On the bundled template A8:B8 is the item number, so column 2 is the
		// item number. Storing "1", "2", "3" would give every month the same
		// positional identity.
		expect(depedLearnerIdFromCells('3', '3')).toBeUndefined();
	});

	test('rejects a name that landed in the ID slot', () => {
		expect(depedLearnerIdFromCells('DELA CRUZ, JUAN', '3')).toBeUndefined();
	});

	test('rejects a value too long to be an ID', () => {
		expect(depedLearnerIdFromCells('x'.repeat(33), '1')).toBeUndefined();
	});
});

describe('matchRosterLearner', () => {
	const existing = [
		roster({ studentId: 'stu-1', rowIndex: 8, sf2LearnerId: '13672845021' }),
		roster({
			studentId: 'stu-2',
			workbookName: 'SANTOS, MARIA',
			normalizedName: 'SANTOS,MARIA',
			rowIndex: 9
		})
	];

	test('the DepEd ID outranks everything, so a reshuffled row still resolves', () => {
		const match = matchRosterLearner(existing, {
			name: 'JUAN DELA CRUZ',
			rowIndex: 30,
			sf2LearnerId: '13672845021'
		});
		expect(match).toMatchObject({ studentId: 'stu-1', matchedBy: 'learnerId' });
	});

	test('the normalized name is the second rule', () => {
		const match = matchRosterLearner(existing, { name: 'santos,  maria', rowIndex: 30 });
		expect(match).toMatchObject({ studentId: 'stu-2', matchedBy: 'normalizedName' });
	});

	test('the row position is the last resort and is labelled as such', () => {
		const match = matchRosterLearner(existing, { name: 'NOBODY, KNOWN', rowIndex: 9 });
		expect(match).toMatchObject({ studentId: 'stu-2', matchedBy: 'rowIndex' });
	});

	test('a learner nobody answers to has no match', () => {
		expect(matchRosterLearner(existing, { name: 'NOBODY, KNOWN', rowIndex: 40 })).toBeUndefined();
	});
});

describe('replaceMonthRoster', () => {
	beforeEach(async () => {
		await insertMonthTemplate({});
	});

	test('replaces the roster in one transaction', async () => {
		await replaceMonthRoster('month-1', [
			roster(),
			roster({
				studentId: 'stu-2',
				workbookName: 'SANTOS, MARIA',
				normalizedName: 'SANTOS,MARIA',
				rowIndex: 9
			})
		]);
		expect(await monthRosterForTemplate('month-1')).toHaveLength(2);

		await replaceMonthRoster('month-1', [
			roster({
				studentId: 'stu-3',
				workbookName: 'REYES, ANA',
				normalizedName: 'REYES,ANA',
				rowIndex: 8
			})
		]);
		const rows = await monthRosterForTemplate('month-1');
		expect(rows).toHaveLength(1);
		expect(rows[0].studentId).toBe('stu-3');
	});

	test('an empty roster is refused, not committed', async () => {
		// Committing it would unmap every learner, and a file whose marks are
		// then written through an empty mapping puts marks on the wrong row.
		await replaceMonthRoster('month-1', [roster()]);
		await expect(replaceMonthRoster('month-1', [])).rejects.toMatchObject({ kind: 'InvalidInput' });
		expect(await monthRosterForTemplate('month-1')).toHaveLength(1);
	});

	test('reads back in workbook row order', async () => {
		await replaceMonthRoster('month-1', [
			roster({ studentId: 'stu-2', normalizedName: 'SANTOS,MARIA', rowIndex: 9 }),
			roster({ studentId: 'stu-1', rowIndex: 8 })
		]);
		expect((await monthRosterForTemplate('month-1')).map((r) => r.studentId)).toEqual([
			'stu-1',
			'stu-2'
		]);
	});
});

describe('roster lookups', () => {
	beforeEach(async () => {
		await insertMonthTemplate({});
		await insertMonthTemplate({ id: 'month-2', reportMonth: 'OCTOBER' });
		await replaceMonthRoster('month-1', [roster({ sf2LearnerId: '13672845021' })]);
		await replaceMonthRoster('month-2', [
			roster({ templateId: 'month-2', rowIndex: 20, sf2LearnerId: '13672845021' })
		]);
	});

	test('every month that knows a DepEd ID is found, whatever row it sits on', async () => {
		const found = await monthMappingsForLearnerId('13672845021');
		expect(found.map((r) => r.templateId)).toEqual(['month-1', 'month-2']);
	});

	test('one normalized name in one month file resolves to one mapping', async () => {
		expect((await monthMappingForNormalizedName('month-1', 'DELA CRUZ,JUAN'))?.studentId).toBe(
			'stu-1'
		);
		expect(await monthMappingForNormalizedName('month-1', 'NOBODY')).toBeUndefined();
	});

	test('deleting one month roster leaves the other month alone', async () => {
		expect(await deleteMonthRoster('month-1')).toBe(1);
		expect(await monthRosterForTemplate('month-1')).toHaveLength(0);
		expect(await monthRosterForTemplate('month-2')).toHaveLength(1);
	});
});

describe('setStudentLearnerIds', () => {
	beforeEach(async () => {
		await db().execute(
			`INSERT INTO students (id, name, created_at) VALUES ('stu-1', 'JUAN', 1), ('stu-2', 'MARIA', 1)`
		);
	});

	test('writes an ID the school gave and skips a blank one', async () => {
		expect(
			await setStudentLearnerIds([
				['stu-1', '13672845021'],
				['stu-2', '   ']
			])
		).toBe(1);
		const row = await db().queryOne<{ sf2_learner_id: string }>(
			'SELECT sf2_learner_id FROM students WHERE id = ?',
			['stu-1']
		);
		expect(row?.sf2_learner_id).toBe('13672845021');
	});

	test('refuses to move one learner ID onto another learner', async () => {
		await db().execute(`UPDATE students SET sf2_learner_id = 'LRN-1' WHERE id = 'stu-1'`);
		expect(await setStudentLearnerIds([['stu-2', 'LRN-1']])).toBe(0);
	});

	test('refuses to overwrite an ID the row already carries', async () => {
		await db().execute(`UPDATE students SET sf2_learner_id = 'LRN-1' WHERE id = 'stu-1'`);
		expect(await setStudentLearnerIds([['stu-1', 'LRN-2']])).toBe(0);
	});

	test('an empty batch writes nothing', async () => {
		expect(await setStudentLearnerIds([])).toBe(0);
	});
});
