/**
 * The import service: the database read that makes the validation report possible,
 * and the row geometry every writer of the same import needs.
 *
 * The geometry is the reason this file exists as arithmetic rather than as five
 * numbers copied at each call site: the TOTAL rows the formulas sum, the rows to
 * hide, and the roster's own rows all shift together when a class grows past the
 * template's 21 male / 19 female slots, and a workbook that disagrees with itself
 * prints a subtotal of nothing.
 */
import { describe, expect, it } from 'vitest';
import { useTestDb } from '$lib/db/repos/__tests__/schema';
import { createClass } from '$lib/db/repos/classes';
import { createStudent } from '$lib/db/repos/students';
import type { Student } from '$lib/domain/models';
import { importValidationFromAnalysis, rosterGeometry } from '../validation-service';
import type { Sf2WorkbookAnalysis } from '../calendar';

useTestDb();

const base = {
	dayStart: '08:00',
	dayEnd: '15:00',
	lateAfter: '08:45',
	room: 'Room 1',
	sessions: [],
	days: []
};

function analysis(overrides: Partial<Sf2WorkbookAnalysis> = {}): Sf2WorkbookAnalysis {
	return {
		schoolId: '',
		schoolName: '',
		schoolYear: '2025-2026',
		reportMonth: 'JUNE',
		gradeLevel: '3',
		section: 'MATAPAT',
		adviserName: '',
		schoolHeadName: '',
		learners: [],
		dates: [],
		sheets: [],
		...overrides
	};
}

function students(male: number, female: number): Student[] {
	const roster: Student[] = [];
	for (let index = 0; index < male; index += 1) {
		roster.push({ id: `m${index}`, name: `Boy, ${index}`, gender: 'male', createdAt: '' });
	}
	for (let index = 0; index < female; index += 1) {
		roster.push({ id: `f${index}`, name: `Girl, ${index}`, gender: 'female', createdAt: '' });
	}
	return roster;
}

describe('importValidationFromAnalysis', () => {
	it('finds the class the workbook names, whatever the casing', async () => {
		const cls = await createClass({ ...base, name: '3 - matapat' });
		await createStudent({
			classId: cls.id,
			name: 'Dela Cruz, Juan',
			gender: 'male',
			cardSerial: undefined
		});

		const report = await importValidationFromAnalysis(
			'C:/book.xlsx',
			analysis({ learners: [{ rowIndex: 8, name: 'Dela Cruz, Juan', genderBlock: 'MALE' }] })
		);

		expect(report.classId).toBe(cls.id);
		expect(report.className).toBe('3 - MATAPAT');
		expect(report.currentStudentCount).toBe(1);
		expect(report.hasDiscrepancies).toBe(false);
	});

	it('compares against nothing when no class has that name yet', async () => {
		await createClass({ ...base, name: '4 - LIGAS' });

		const report = await importValidationFromAnalysis(
			'C:/book.xlsx',
			analysis({ learners: [{ rowIndex: 8, name: 'Dela Cruz, Juan', genderBlock: 'MALE' }] })
		);

		expect(report.classId).toBeUndefined();
		expect(report.currentStudentCount).toBe(0);
		expect(report.missingFromCurrent).toEqual([]);
		expect(report.hasDiscrepancies).toBe(false);
	});

	it('reports a learner the class does not have', async () => {
		const cls = await createClass({ ...base, name: '3 - MATAPAT' });
		await createStudent({
			classId: cls.id,
			name: 'Dela Cruz, Juan',
			gender: 'male',
			cardSerial: undefined
		});

		const report = await importValidationFromAnalysis(
			'C:/book.xlsx',
			analysis({ learners: [{ rowIndex: 8, name: 'Santos, Maria', genderBlock: 'FEMALE' }] })
		);

		expect(report.missingFromCurrent.map((entry) => entry.name)).toEqual(['Santos, Maria']);
		expect(report.missingFromSf2.map((entry) => entry.name)).toEqual(['Dela Cruz, Juan']);
		expect(report.hasDiscrepancies).toBe(true);
	});
});

describe('rosterGeometry', () => {
	it('leaves the template rows where they are for a class that fits', () => {
		const geometry = rosterGeometry(students(12, 14), [8, 9]);

		expect(geometry.maleCount).toBe(12);
		expect(geometry.femaleCount).toBe(14);
		expect(geometry.extraMale).toBe(0);
		expect(geometry.extraFemale).toBe(0);
		expect(geometry.maleTotalRow).toBe(29);
		expect(geometry.femaleTotalRow).toBe(49);
		expect(geometry.combinedTotalRow).toBe(50);
		expect(geometry.hideThroughMaleRow).toBe(29);
		expect(geometry.hideThroughFemaleRow).toBe(49);
		expect([...geometry.occupiedRows]).toEqual([8, 9]);
	});

	it('grows both blocks together when the class is larger than the template', () => {
		const geometry = rosterGeometry(students(25, 25), [8, 9]);

		expect(geometry.extraMale).toBe(4);
		expect(geometry.extraFemale).toBe(6);
		// 8 + 25 male slots, then 30 + 4 extra male rows of female slots.
		expect(geometry.maleTotalRow).toBe(33);
		expect(geometry.femaleTotalRow).toBe(59);
		expect(geometry.combinedTotalRow).toBe(60);
	});

	it('shifts the rows to hide by exactly the growth, so no slot is left showing', () => {
		const grown = rosterGeometry(students(25, 25), []);
		const fitting = rosterGeometry(students(12, 14), []);

		expect(grown.hideThroughMaleRow - fitting.hideThroughMaleRow).toBe(4);
		expect(grown.hideThroughFemaleRow - fitting.hideThroughFemaleRow).toBe(10);
		expect(grown.hideThroughFemaleRow).toBe(grown.femaleTotalRow);
	});

	it('counts a student with no recorded gender in neither block', () => {
		const geometry = rosterGeometry([{ id: 'x', name: 'Nobody, Known', createdAt: '' }], []);
		expect(geometry.maleCount).toBe(0);
		expect(geometry.femaleCount).toBe(0);
		expect(geometry.maleTotalRow).toBe(29);
	});
});
