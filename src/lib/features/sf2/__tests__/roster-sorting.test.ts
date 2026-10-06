import { describe, expect, test } from 'vitest';
import { compareLearnerNames, normalizeLearnerName } from '../logic';
import { templateRosterAssignments } from '../roster';
import type { Student } from '$lib/domain/models';

function student(id: string, name: string, gender: 'male' | 'female'): Student {
	return { id, name, gender, createdAt: '2026-01-01T00:00:00.000Z' };
}

describe('compareLearnerNames', () => {
	test('sorts A-Z on the normalized name', () => {
		expect(compareLearnerNames('REYES, ANA', 'SANTOS, MARIA')).toBeLessThan(0);
		expect(compareLearnerNames('SANTOS, MARIA', 'REYES, ANA')).toBeGreaterThan(0);
		expect(compareLearnerNames('SANTOS, MARIA', 'SANTOS, MARIA')).toBe(0);
	});

	test('folds case and spacing so equal names compare equal', () => {
		expect(compareLearnerNames('dela Cruz,  Juan', 'DELA CRUZ,JUAN')).toBe(0);
	});

	test('agrees with normalizeLearnerName identity', () => {
		const names = ['SANTOS, MARIA', 'dela cruz, juan', 'REYES, ANA'];
		const sorted = [...names].sort(compareLearnerNames);
		expect(sorted.map(normalizeLearnerName)).toEqual(
			[...sorted.map(normalizeLearnerName)].sort()
		);
	});
});

describe('templateRosterAssignments alphabetical order', () => {
	test('sorts males A-Z and females A-Z in separate blocks', () => {
		const assignments = templateRosterAssignments([
			student('m2', 'SANTOS, PEDRO', 'male'),
			student('f2', 'SANTOS, MARIA', 'female'),
			student('m1', 'DELA CRUZ, JUAN', 'male'),
			student('f1', 'REYES, ANA', 'female')
		]);
		expect(assignments.map((entry) => entry.student.name)).toEqual([
			'DELA CRUZ, JUAN',
			'SANTOS, PEDRO',
			'REYES, ANA',
			'SANTOS, MARIA'
		]);
		// Male block rows 8.. then female block rows 30...
		expect(assignments.map((entry) => entry.slot.rowIndex)).toEqual([8, 9, 30, 31]);
		expect(assignments.map((entry) => entry.slot.genderBlock)).toEqual([
			'MALE',
			'MALE',
			'FEMALE',
			'FEMALE'
		]);
	});

	test('a mid-alphabet insert lands mid-block, not at the end', () => {
		const assignments = templateRosterAssignments([
			student('m1', 'AQUINO, JOSE', 'male'),
			student('m2', 'ZAMORA, RICO', 'male'),
			student('m3', 'MENDOZA, LITO', 'male')
		]);
		expect(assignments.map((entry) => entry.student.name)).toEqual([
			'AQUINO, JOSE',
			'MENDOZA, LITO',
			'ZAMORA, RICO'
		]);
	});
});
