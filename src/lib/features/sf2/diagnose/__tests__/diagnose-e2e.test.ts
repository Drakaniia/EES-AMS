import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { diagnoseSf2Marks } from '../diagnose';
import {
	CLASS_ID,
	clearMarks,
	MALE_TWO,
	SEPTEMBER_2026_DAYS,
	mountInstall,
	seedInstall,
	septemberWorkbook,
	unmountInstall,
	type Install
} from './install-fixture';

let install: Install;

beforeEach(async () => {
	install = await mountInstall();
});

afterEach(async () => {
	await unmountInstall(install);
});

/** The September month of the current run, or a loud failure. */
function septemberMonth(diagnostic: Awaited<ReturnType<typeof diagnoseSf2Marks>>) {
	const month = diagnostic.months.find((candidate) => candidate.reportMonth === 'SEPTEMBER');
	if (month === undefined) throw new Error('no September in the diagnostic');
	return month;
}
describe('an install whose workbook and database agree', () => {
	it('measures the month and says the database holds every mark', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();

		const month = septemberMonth(diagnostic);
		expect(month.sourceStatus).toBe('Comparable');
		// Two workbook X marks and no database absences on a grid column: the database is
		// missing them, which is the answer this whole feature exists to give.
		expect(month.counts.workbookXCount).toBe(3);
		// The seed's absences already cover two of the three marks, so exactly one is
		// missing from the database and one extra absence is missing from the workbook.
		expect(month.cellsOnlyInWorkbook).toHaveLength(1);
		expect(month.cellsOnlyInDatabase).toHaveLength(1);
		expect(diagnostic.verdictReason).toContain('1 X mark(s)');
	});

	it('names each missing mark by learner, day and address', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const month = septemberMonth(await diagnoseSf2Marks());

		expect(month.cellsOnlyInWorkbook.map((cell) => cell.cellAddress)).toEqual(['I9']);
		expect(month.cellsOnlyInWorkbook[0]?.date).toBe('2026-09-03');
		expect(month.cellsOnlyInWorkbook[0]?.studentName).toBe(MALE_TWO);
		expect(month.cellsOnlyInDatabase.map((cell) => cell.cellAddress)).toEqual(['H8']);
		expect(month.cellsOnlyInDatabase[0]?.date).toBe('2026-09-02');
		expect(month.cellsOnlyInWorkbook.every((cell) => cell.studentName !== '')).toBe(true);
	});

	it('reports the workbook and the class it is anchored on', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();

		expect(diagnostic.workbookPath).toBe(install.fixture.path);
		expect(diagnostic.workbookDir).toBe('/Documents/EES-AMS/workbooks');
		expect(diagnostic.activeClassId).toBe(CLASS_ID);
		expect(diagnostic.schoolYear).toBe('2026 - 2027');
		expect(diagnostic.storedReportMonth).toBe('SEPTEMBER');
		expect(diagnostic.mappingSource).toBe('perMonthTables');
		expect(diagnostic.schemaVersion).toBe(19);
	});

	it('reads the month from the worksheet, not from the stored grid', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const month = septemberMonth(await diagnoseSf2Marks());

		// 22 day columns printed by this fixture; the database's stored grid names only
		// two. The file is the authority, because the stored grid is what a destructive
		// sync destroys.
		expect(month.dayColumns).toBe(SEPTEMBER_2026_DAYS.length);
		expect(month.rosterRows).toBe(3);
		expect(month.rosterResolution).toBe('databaseRowMappings');
	});

	it('reports the eleven months it could not measure', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();

		expect(diagnostic.months).toHaveLength(12);
		expect(diagnostic.incomparableMonths).toHaveLength(11);
		expect(diagnostic.incomparableMonths[0]).toMatch(/^JANUARY \d+: NoSheet - No worksheet/);
	});

	it('counts absences the grid can hold separately from the ones it cannot', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver, {
			absences: [
				{ id: 'e1', studentId: 's1', classId: CLASS_ID, date: '2026-09-01' },
				// 2026-09-05 is a Saturday: the grid has no column for it.
				{ id: 'e2', studentId: 's1', classId: CLASS_ID, date: '2026-09-05' }
			]
		});

		const month = septemberMonth(await diagnoseSf2Marks());

		expect(month.counts.dbAbsentCount).toBe(2);
		expect(month.counts.dbMappedAbsentCount).toBe(1);
	});

	it('reports absences belonging to no class this workbook knows about', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();

		// Four absences in the database, three attributable to this class.
		expect(diagnostic.totalAbsentEvents).toBe(4);
		expect(diagnostic.absentEventsWithoutClass).toBe(1);
	});

	it('says the database holds every mark when it does', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver, {
			absences: [
				{ id: 'e1', studentId: 's1', classId: CLASS_ID, date: '2026-09-01' },
				{ id: 'e2', studentId: 's2', classId: CLASS_ID, date: '2026-09-03' },
				{ id: 'e3', studentId: 's1', classId: CLASS_ID, date: '2026-09-07' }
			]
		});

		const month = septemberMonth(await diagnoseSf2Marks());

		expect(month.cellsOnlyInWorkbook).toHaveLength(0);
		expect(month.reason).toBe('The database holds every X the workbook shows, on the same cells.');
		expect(month.counts.workbookXCount).toBe(3);
	});
});

describe('a month with nothing on it at all', () => {
	it('reports a measured zero rather than refusing to answer', async () => {
		// The no-cry-wolf case. A month nobody was absent is a real measurement of
		// zero; reporting it as unreadable would make every honest install look broken.
		await septemberWorkbook(install.fixture, (_workbook, september) => clearMarks(september));
		await seedInstall(install.driver, { absences: [] });

		const month = septemberMonth(await diagnoseSf2Marks());

		expect(month.sourceStatus).toBe('Comparable');
		expect(month.counts.workbookXCount).toBe(0);
		expect(month.counts.dbAbsentCount).toBe(0);
		expect(month.cellsOnlyInWorkbook).toHaveLength(0);
		expect(month.cellsOnlyInDatabase).toHaveLength(0);
		expect(month.reason).toBe('The database holds every X the workbook shows, on the same cells.');
	});

	it('reports every day column it looked at, so a zero is auditable', async () => {
		await septemberWorkbook(install.fixture, (_workbook, september) => clearMarks(september));
		await seedInstall(install.driver, { absences: [] });

		const month = septemberMonth(await diagnoseSf2Marks());

		// Three roster rows x 22 day columns. A zero over zero cells would be a refusal
		// wearing a number.
		expect(month.counts.cellsScanned).toBe(3 * SEPTEMBER_2026_DAYS.length);
	});
});
