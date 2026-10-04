/**
 * `sf2::preview::export_preview` — the cell status, the gender label, and the
 * grid a month read renders.
 *
 * These tests drive {@link buildExportPreview} with hand-built data rather than a
 * database, because the module takes no database: that is the property being
 * asserted, not an accident of the fixture.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	buildExportPreview,
	previewCellStatus,
	previewGender,
	registerSf2Preview
} from '../preview';
import type { MonthGridInput } from '$lib/features/sf2/month/month';
import type { Sf2ExportReadiness, Sf2TemplateSummary } from '$lib/types';

/**
 * The month service's registration seam, observed rather than exercised.
 *
 * `buildExportPreview` takes only a *type* from that module, so mocking it costs
 * nothing and lets the registration be asserted directly — which is the whole
 * claim: after startup the month service holds *this* function, so there is one
 * implementation of "an X in the grid means an absent record".
 */
const { installed } = vi.hoisted(() => ({
	installed: { current: null as unknown }
}));

vi.mock('$lib/features/sf2/month/month', () => ({
	useMonthGridBuilder: (builder: unknown) => {
		installed.current = builder;
	}
}));

beforeEach(() => {
	installed.current = null;
});

const SHEET = 'SEPTEMBER 2026';

const TEMPLATE: Sf2TemplateSummary = {
	id: 'month-1',
	sourcePath: 'C:/workbooks/SF2-GRADE-3-MATAPAT.xlsx',
	schoolId: '',
	schoolName: 'Matapat Elementary School',
	schoolYear: '2026-2027',
	reportMonth: 'SEPTEMBER',
	gradeLevel: '3',
	section: 'MATAPAT',
	adviserName: '',
	schoolHeadName: '',
	classId: 'class-1',
	importedAt: 0
};

function readiness(overrides: Partial<Sf2ExportReadiness> = {}): Sf2ExportReadiness {
	return {
		mappedStudents: 2,
		mappedDates: 2,
		canExport: true,
		issues: [],
		warnings: [],
		...overrides
	};
}

/** A local instant, because every date rule in the SF2 grid is a local-calendar rule. */
function at(year: number, month: number, day: number, hour = 9): string {
	return new Date(year, month - 1, day, hour, 0, 0).toISOString();
}

function input(overrides: Partial<MonthGridInput> = {}): MonthGridInput {
	return {
		template: TEMPLATE,
		roster: [
			{
				templateId: 'month-1',
				studentId: 's1',
				workbookName: 'Dela Cruz, Juan',
				normalizedName: 'DELA CRUZ,JUAN',
				rowIndex: 8,
				genderBlock: 'MALE'
			},
			{
				templateId: 'month-1',
				studentId: 's2',
				workbookName: 'Reyes, Maria',
				normalizedName: 'REYES,MARIA',
				rowIndex: 30,
				genderBlock: 'FEMALE'
			}
		],
		dates: [
			{ date: '2026-09-01', sheetName: SHEET, columnLetter: 'F', columnIndex: 6 },
			{ date: '2026-09-02', sheetName: SHEET, columnLetter: 'H', columnIndex: 8 }
		],
		className: 'Grade 3 - Matapat',
		classStudents: [
			{ id: 's1', name: 'Dela Cruz, Juan', gender: 'male', createdAt: '' },
			{ id: 's2', name: 'Reyes, Maria', gender: 'female', createdAt: '' }
		],
		events: [],
		readiness: readiness(),
		...overrides
	};
}

describe('previewCellStatus', () => {
	it('is present when the day has attendance and the learner has no absence', () => {
		expect(previewCellStatus(false, true)).toBe('present');
	});

	it('is absent when the learner has an explicit absent record', () => {
		expect(previewCellStatus(true, true)).toBe('absent');
	});

	it('is absent even on a day with no other record', () => {
		expect(previewCellStatus(true, false)).toBe('absent');
	});

	it('is open when nothing was recorded that day', () => {
		expect(previewCellStatus(false, false)).toBe('open');
	});
});

describe('previewGender', () => {
	it('prefers the gender the database holds', () => {
		expect(previewGender('female', 'MALE')).toBe('female');
	});

	it('falls back to the roster block, labelled the way the form labels it', () => {
		expect(previewGender(undefined, 'FEMALE')).toBe('Female');
		expect(previewGender('  ', 'male')).toBe('Male');
	});

	it('passes an unrecognised block through rather than guessing', () => {
		expect(previewGender(undefined, 'UNLISTED')).toBe('UNLISTED');
		expect(previewGender(undefined, undefined)).toBeUndefined();
		expect(previewGender(undefined, '  ')).toBeUndefined();
	});
});

describe('buildExportPreview', () => {
	it('marks a learner absent from an explicit absent record alone', () => {
		const preview = buildExportPreview(
			input({
				events: [
					{
						id: 'e1',
						studentId: 's1',
						classId: 'class-1',
						type: 'absent',
						timestamp: at(2026, 9, 1)
					}
				]
			})
		);

		const row = preview.students.find((entry) => entry.studentId === 's1');
		expect(row?.cells[0]).toEqual({ date: '2026-09-01', status: 'absent', editable: true });
		expect(preview.absenceCount).toBe(1);
		expect(preview.absentList).toEqual([
			{ studentId: 's1', studentName: 'Dela Cruz, Juan', date: '2026-09-01', rowIndex: 8 }
		]);
	});

	it('leaves every cell open until attendance is taken', () => {
		const preview = buildExportPreview(input());
		expect(preview.students.every((row) => row.cells.every((cell) => cell.status === 'open'))).toBe(
			true
		);
		expect(preview.presentCount).toBe(0);
	});

	it('counts a day as taken from an `in` record and calls the rest of it present', () => {
		const preview = buildExportPreview(
			input({
				events: [
					{ id: 'e1', studentId: 's1', classId: 'class-1', type: 'in', timestamp: at(2026, 9, 1) }
				]
			})
		);

		expect(preview.students.find((r) => r.studentId === 's1')?.cells[0].status).toBe('present');
		// Present by default: one absence marks one learner, and the other is untouched.
		expect(preview.students.find((r) => r.studentId === 's2')?.cells[0].status).toBe('present');
		expect(preview.presentCount).toBe(2);
	});

	it('ignores a record stamped on another day', () => {
		const preview = buildExportPreview(
			input({
				events: [
					{
						id: 'e1',
						studentId: 's1',
						classId: 'class-1',
						type: 'absent',
						timestamp: at(2026, 9, 3)
					}
				]
			})
		);
		expect(preview.absenceCount).toBe(0);
	});

	it('uses the local day, so an evening mark is not filed under tomorrow', () => {
		const preview = buildExportPreview(
			input({
				events: [
					{
						id: 'e1',
						studentId: 's1',
						classId: 'class-1',
						type: 'absent',
						timestamp: at(2026, 9, 1, 23)
					}
				]
			})
		);
		expect(preview.absentList[0].date).toBe('2026-09-01');
	});

	it('counts an event with no class when the learner is on the roster', () => {
		const preview = buildExportPreview(
			input({
				events: [{ id: 'e1', studentId: 's1', type: 'absent', timestamp: at(2026, 9, 1) }]
			})
		);
		expect(preview.absenceCount).toBe(1);
	});

	it('ignores an event for a learner who is not on the roster', () => {
		const preview = buildExportPreview(
			input({
				events: [
					{
						id: 'e1',
						studentId: 'stranger',
						classId: 'class-1',
						type: 'absent',
						timestamp: at(2026, 9, 1)
					}
				]
			})
		);
		expect(preview.absenceCount).toBe(0);
	});

	it('drops a mapped row whose student was deleted instead of showing it', () => {
		const preview = buildExportPreview(
			input({
				classStudents: [{ id: 's2', name: 'Reyes, Maria', gender: 'female', createdAt: '' }]
			})
		);

		expect(preview.students.find((entry) => entry.studentId === 's1')).toBeUndefined();
		expect(preview.students.map((entry) => entry.studentId)).toEqual(['s2']);
		expect(preview.warnings).not.toContain(
			'Dela Cruz, Juan is mapped in the SF2 workbook but is not in the selected class.'
		);
	});

	it('lists an unmapped class learner as present, uneditable, on row 0', () => {
		const preview = buildExportPreview(
			input({
				classStudents: [
					...input().classStudents,
					{ id: 's3', name: 'Bautista, Liza', gender: 'female', createdAt: '' }
				]
			})
		);

		const row = preview.students.find((entry) => entry.studentId === 's3');
		expect(row).toMatchObject({
			mapped: false,
			rowIndex: 0,
			workbookName: '',
			presentCount: 0,
			absentCount: 0
		});
		expect(row?.cells.every((cell) => cell.status === 'present' && cell.editable === false)).toBe(
			true
		);
		expect(preview.unmappedStudentCount).toBe(1);
		expect(preview.warnings).toContain(
			'Bautista, Liza is in the class roster but is not mapped to an SF2 learner row.'
		);
	});

	it('says so when nothing is mapped, rather than showing an empty grid', () => {
		const preview = buildExportPreview(input({ roster: [] }));
		expect(preview.warnings).toContain('No learners are mapped to this SF2 workbook.');
		expect(preview.students).toHaveLength(2);
	});

	it('echoes the readiness issues rather than recomputing them', () => {
		const preview = buildExportPreview(
			input({
				readiness: readiness({ issues: ['No SF2 workbook is stored yet.'], canExport: false })
			})
		);
		expect(preview.issues).toEqual(['No SF2 workbook is stored yet.']);
		expect(preview.canExport).toBe(false);
	});

	it('is deterministic: the same input gives the same row and warning order', () => {
		const events = [
			{
				id: 'e1',
				studentId: 's1',
				classId: 'class-1',
				type: 'absent' as const,
				timestamp: at(2026, 9, 1)
			},
			{
				id: 'e2',
				studentId: 's2',
				classId: 'class-1',
				type: 'absent' as const,
				timestamp: at(2026, 9, 2)
			}
		];
		const first = buildExportPreview(input({ events }));
		const second = buildExportPreview(input({ events }));
		expect(second.students.map((r) => r.studentId)).toEqual(first.students.map((r) => r.studentId));
		expect(second.warnings).toEqual(first.warnings);
		expect(second.absentList).toEqual(first.absentList);
	});
});

describe('registerSf2Preview', () => {
	it("installs this module's builder as the month service's grid builder", () => {
		registerSf2Preview();
		// Identity, not a behaviour check: the point is that the month service now
		// holds *this* function rather than a second copy of the same rules.
		expect(installed.current).toBe(buildExportPreview);
	});
});
