/**
 * The roster validation report.
 *
 * This is the last thing standing between a teacher's workbook and the database,
 * so the cases below are the ones where a wrong answer costs real data: a learner
 * the class has and the workbook does not, the same learner spelled two ways, a
 * duplicate row, and the fresh-install case where the class has nobody yet.
 */
import { describe, expect, it } from 'vitest';
import type { Student } from '$lib/domain/models';
import {
	editDistance,
	ensureImportValidationAllows,
	analysisClassName,
	nameMismatchReason,
	validateStudentList
} from '../validation';
import type { Sf2WorkbookAnalysis, Sf2WorkbookLearner } from '../calendar';

function student(id: string, name: string, gender?: 'male' | 'female'): Student {
	return { id, name, createdAt: '2026-01-01T00:00:00Z', ...(gender ? { gender } : {}) };
}

function learner(rowIndex: number, name: string, genderBlock?: string): Sf2WorkbookLearner {
	return { rowIndex, name, ...(genderBlock ? { genderBlock } : {}) };
}

const AGREEING: [Student[], Sf2WorkbookLearner[]] = [
	[student('s1', 'Dela Cruz, Juan', 'male'), student('s2', 'Reyes, Ana', 'female')],
	[learner(8, 'Dela Cruz, Juan', 'MALE'), learner(30, 'Reyes, Ana', 'FEMALE')]
];

describe('validateStudentList', () => {
	it('reports no discrepancy when both sides agree', () => {
		const report = validateStudentList('C:/book.xlsx', 'c1', '3 - A', ...AGREEING);
		expect(report.hasDiscrepancies).toBe(false);
		expect(report.currentStudentCount).toBe(2);
		expect(report.sf2LearnerCount).toBe(2);
		expect(report.missingFromSf2).toEqual([]);
		expect(report.missingFromCurrent).toEqual([]);
	});

	it('names a class student the workbook does not have', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[student('s1', 'Dela Cruz, Juan', 'male'), student('s2', 'Reyes, Ana', 'female')],
			[learner(8, 'Dela Cruz, Juan', 'MALE')]
		);
		expect(report.missingFromSf2.map((entry) => entry.studentId)).toEqual(['s2']);
		expect(report.hasDiscrepancies).toBe(true);
	});

	it('names a workbook learner the class does not have', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[student('s1', 'Dela Cruz, Juan', 'male')],
			[learner(8, 'Dela Cruz, Juan', 'MALE'), learner(9, 'Santos, Maria', 'MALE')]
		);
		expect(report.missingFromCurrent.map((entry) => entry.rowIndex)).toEqual([9]);
	});

	it('treats a class with no students as agreeing with everything', () => {
		// A fresh install's first import must not open on a report full of learners
		// "missing from current".
		const report = validateStudentList(
			'C:/book.xlsx',
			undefined,
			'3 - A',
			[],
			[learner(8, 'Dela Cruz, Juan', 'MALE')]
		);
		expect(report.missingFromCurrent).toEqual([]);
		expect(report.hasDiscrepancies).toBe(false);
		expect(report.classId).toBeUndefined();
	});

	it('ignores the form rows the workbook carries', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[],
			[
				learner(5, 'NAME (Last Name, First Name, Middle Name)'),
				learner(29, 'MALE TOTAL'),
				learner(50, 'COMBINED TOTAL')
			]
		);
		expect(report.sf2LearnerCount).toBe(0);
		expect(report.hasDiscrepancies).toBe(false);
	});

	it('flags a learner row with no gender block, which the roster cannot place', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[],
			[learner(8, 'Dela Cruz, Juan')]
		);
		expect(report.missingLearnerInfo.map((entry) => entry.rowIndex)).toEqual([8]);
		expect(report.hasDiscrepancies).toBe(true);
	});

	it('flags a blank learner row', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[],
			[{ rowIndex: 9, name: '   ' }]
		);
		expect(report.missingLearnerInfo).toHaveLength(1);
		expect(report.hasDiscrepancies).toBe(true);
	});

	it('reports duplicate rows on both sides, keeping the ids and the rows apart', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[student('s1', 'Dela Cruz, Juan'), student('s2', 'dela cruz, juan')],
			[learner(8, 'Dela Cruz, Juan', 'MALE'), learner(9, 'Dela Cruz, Juan', 'MALE')]
		);
		expect(report.duplicateCurrentStudents).toEqual([
			{
				normalizedName: 'DELA CRUZ,JUAN',
				names: ['Dela Cruz, Juan', 'dela cruz, juan'],
				studentIds: ['s1', 's2'],
				rowIndexes: []
			}
		]);
		expect(report.duplicateSf2Learners).toEqual([
			{
				normalizedName: 'DELA CRUZ,JUAN',
				names: ['Dela Cruz, Juan', 'Dela Cruz, Juan'],
				studentIds: [],
				rowIndexes: [8, 9]
			}
		]);
	});

	it('matches a name however it is spaced or capitalised', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[student('s1', 'Dela  Cruz,  Juan', 'male')],
			[learner(8, ' DELA CRUZ,JUAN ', 'MALE')]
		);
		expect(report.hasDiscrepancies).toBe(false);
	});
});

describe('possible name mismatches', () => {
	it('sees the same tokens in another order', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[student('s1', 'Dela Cruz, Juan', 'male')],
			[learner(8, 'Juan, Dela Cruz', 'MALE')]
		);
		expect(report.possibleNameMismatches).toHaveLength(1);
		expect(report.possibleNameMismatches[0].reason).toBe('Same name tokens in a different order');
	});

	it('sees a spelling one or two edits away', () => {
		expect(nameMismatchReason('DELA CRUZ, JUAN', 'DELA CRUZ, JUN')).toBe('Very similar spelling');
	});

	it('leaves two different people alone', () => {
		expect(nameMismatchReason('DELA CRUZ, JUAN', 'SANTOS, MARIA')).toBeUndefined();
		expect(nameMismatchReason('JUAN', 'PEDRO')).toBeUndefined();
	});

	it('measures edit distance the way the validator does', () => {
		expect(editDistance('', 'abc')).toBe(3);
		expect(editDistance('kitten', 'sitting')).toBe(3);
		expect(editDistance('same', 'same')).toBe(0);
	});
});

describe('ensureImportValidationAllows', () => {
	const clean = validateStudentList('C:/book.xlsx', 'c1', '3 - A', ...AGREEING);
	const dirty = validateStudentList(
		'C:/book.xlsx',
		'c1',
		'3 - A',
		[student('s9', 'Nobody, Here')],
		[learner(8, 'Dela Cruz, Juan', 'MALE')]
	);

	it('refuses a mismatch until the teacher says to go ahead', () => {
		expect(() => ensureImportValidationAllows(dirty, false)).toThrow();
	});

	it('lets an acknowledged mismatch through', () => {
		expect(() => ensureImportValidationAllows(dirty, true)).not.toThrow();
	});

	it('lets a clean report through either way', () => {
		expect(() => ensureImportValidationAllows(clean, false)).not.toThrow();
		expect(() => ensureImportValidationAllows(clean, true)).not.toThrow();
	});
});

describe('analysisClassName', () => {
	it('names the class the workbook grade and section imply', () => {
		const analysis = {
			gradeLevel: ' 3 ',
			section: 'MATAPAT '
		} as Sf2WorkbookAnalysis;
		expect(analysisClassName(analysis)).toBe('3 - MATAPAT');
	});

	it('falls back to a name when the workbook names neither', () => {
		const analysis = { gradeLevel: '', section: '' } as Sf2WorkbookAnalysis;
		expect(analysisClassName(analysis)).toBe('SF2 Class');
	});
});

describe('the learner shape', () => {
	it('keeps the row index and gender block the report shows', () => {
		const report = validateStudentList(
			'C:/book.xlsx',
			'c1',
			'3 - A',
			[],
			[learner(30, 'Reyes, Ana'), learner(31, 'Santos, Maria', 'FEMALE')]
		);
		expect(report.missingLearnerInfo).toEqual([
			{
				rowIndex: 30,
				name: 'Reyes, Ana',
				normalizedName: 'REYES,ANA',
				genderBlock: undefined
			}
		]);
	});
});
