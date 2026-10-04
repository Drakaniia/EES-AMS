import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { diagnoseSf2Marks } from '../diagnose';
import {
	addWorkbook,
	CLASS_ID,
	clearMarks,
	dayColumns,
	FEMALE_ROW,
	hideSheet,
	LEGACY_WORKBOOK_PATH,
	localDateOf,
	MALE_ROW,
	MALE_TOTAL_ROW,
	mountInstall,
	printDayRow,
	renameSheet,
	SEPTEMBER_2026_DAYS,
	SEPTEMBER_SHEET,
	seedInstall,
	septemberWorkbook,
	snapshotTables,
	unmountInstall,
	writeX,
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
describe('a month nobody could place', () => {
	it('refuses a sheet whose day row is empty, and says the marks are there', async () => {
		await septemberWorkbook(install.fixture, (_workbook, september) => printDayRow(september, []));
		await seedInstall(install.driver);

		const month = septemberMonth(await diagnoseSf2Marks());

		expect(month.sourceStatus).toBe('NoMappings');
		expect(month.counts.workbookXCount).toBeUndefined();
		expect(month.counts.dbAbsentCount).toBeUndefined();
		expect(month.reason).toContain('its day-number row is empty');
		expect(month.reason).toContain('The marks are there; nothing can place them.');
		expect(month.sheetName).toBe(SEPTEMBER_SHEET);
	});

	it('refuses a month no worksheet anywhere names', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();
		const august = diagnostic.months.find((month) => month.reportMonth === 'AUGUST');

		expect(august?.sourceStatus).toBe('NoSheet');
		expect(august?.counts.workbookXCount).toBeUndefined();
		expect(august?.reason).toContain('No worksheet in any of');
		expect(august?.reason).toContain('which is not the same as it holding no marks');
	});

	it('refuses a month the roster cannot place a learner on', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);
		await install.driver.execute('DELETE FROM sf2_month_student_mappings');
		await install.driver.execute('DELETE FROM sf2_student_mappings');

		const month = septemberMonth(await diagnoseSf2Marks());

		expect(month.sourceStatus).toBe('NoMappings');
		expect(month.reason).toContain('no learner could be placed on a row');
		expect(month.counts.workbookXCount).toBeUndefined();
	});

	it('keeps a worksheet nobody placed visible instead of reporting it as empty', async () => {
		// A hidden worksheet left behind by the old calendar cycle, holding a mark and
		// naming no month. Nothing can place it, and it must still be visible.
		await septemberWorkbook(install.fixture, (workbook) => {
			const orphan = workbook.worksheets[1]!;
			writeX(orphan, FEMALE_ROW, dayColumns(orphan)[0]!);
		});
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();

		expect(diagnostic.unplacedSheets).toHaveLength(1);
		expect(diagnostic.unplacedSheets[0]?.visible).toBe(false);
		expect(diagnostic.unplacedSheets[0]?.marks).toHaveLength(1);
		expect(diagnostic.unplacedSheets[0]?.reason).toContain('was not used for a month');
		expect(diagnostic.unplacedSheets[0]?.unrowedXCount).toBe(1);
		// The workbook scan counts it, and no month compared it, so it is visible in
		// exactly one place: the unplaced list.
		expect(diagnostic.workbooks[0]?.totalXCount).toBe(4);
		expect(septemberMonth(diagnostic).cellsOnlyInWorkbook).toHaveLength(1);
	});

	it('says why a nameless sheet cannot be placed', async () => {
		await septemberWorkbook(install.fixture, (workbook) => {
			const orphan = workbook.worksheets[1]!;
			printDayRow(orphan, []);
			writeX(orphan, MALE_ROW + 4, dayColumns(orphan)[0]!);
		});
		await seedInstall(install.driver);

		const unplaced = (await diagnoseSf2Marks()).unplacedSheets;

		expect(unplaced[0]?.reason).toContain('its name says no month');
		expect(unplaced[0]?.marks[0]?.date).toBe('(no day grid on this sheet)');
	});
});

describe('a workbook the database points at that is not there', () => {
	it('refuses every month rather than reporting a zero', async () => {
		// The database names a workbook in a directory with nothing in it, which is the
		// ordinary "no workbook yet" shape.
		await seedInstall(install.driver, { sourcePath: '/Documents/EES-AMS/elsewhere/absent.xlsx' });

		const diagnostic = await diagnoseSf2Marks();

		expect(diagnostic.verdict).toBe('Incomparable');
		expect(diagnostic.workbooks).toHaveLength(0);
		expect(diagnostic.months.every((month) => month.sourceStatus === 'WorkbookMissing')).toBe(true);
		expect(diagnostic.months.every((month) => month.counts.workbookXCount === undefined)).toBe(
			true
		);
		expect(diagnostic.verdictReason).toContain('12 of 12 month(s) could not be measured');
	});

	it('measures the month another file covers rather than refusing it', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver, { sourcePath: '/Documents/EES-AMS/workbooks/absent.xlsx' });

		const diagnostic = await diagnoseSf2Marks();

		// The named file is gone but a real workbook sits in the directory and is measured.
		// Saying `WorkbookMissing` for a month a readable file covers would be the false
		// zero this module exists to avoid.
		expect(septemberMonth(diagnostic).sourceStatus).toBe('Comparable');
		expect(diagnostic.workbooks[0]?.isReferencedByDatabase).toBe(false);
	});
});

describe('a file in the workbook directory that will not open', () => {
	it('names the file and does not claim the month has no sheet', async () => {
		await septemberWorkbook(install.fixture);
		await install.fixture.fileSystem.writeFileAtomic(
			'/Documents/EES-AMS/workbooks/SF2-GRADE-3-MATAPAT-0000t1.xlsx',
			'this is not a workbook'
		);
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();

		expect(diagnostic.workbooks.map((workbook) => workbook.path)).toContain(
			'/Documents/EES-AMS/workbooks/SF2-GRADE-3-MATAPAT-0000t1.xlsx'
		);
		const unreadable = diagnostic.workbooks.find((workbook) => workbook.readError !== undefined);
		expect(unreadable?.readError).toBeDefined();
		// The month the readable file covers is still measured: the whole difference
		// between "the user has the file open" and "the tool failed".
		expect(septemberMonth(diagnostic).sourceStatus).toBe('Comparable');
		// And the month nobody's readable file covers says so, rather than saying the
		// workbook has no sheet for it.
		const august = diagnostic.months.find((month) => month.reportMonth === 'AUGUST');
		expect(august?.sourceStatus).toBe('ExcelUnavailable');
		expect(august?.reason).toContain('is not a statement about');
	});

	it('measures a month that only lives in a second file in the directory', async () => {
		// The Rust fixture comment on this: two of the four files in a real install have no
		// extension at all, and one of them holds the only `AUGUST` sheet. A probe that only
		// opened the file the database points at, or that filtered on `.xls`, would have
		// reported August as having no sheet - which is how real marks get hidden.
		await septemberWorkbook(install.fixture);
		await addWorkbook(install.fixture, LEGACY_WORKBOOK_PATH, (workbook) => {
			// The copy is made from the already-built workbook, so the second file also
			// carries the September sheet. That is realistic and harmless: the referenced
			// file is probed first, so September still resolves to it.
			const august = workbook.worksheets[1]!;
			renameSheet(august, 'AUGUST 2026');
			printDayRow(august, SEPTEMBER_2026_DAYS);
			clearMarks(august);
			// The working copy spells the learner slightly differently, so only a name
			// match can line it up with the database.
			august.getRow(MALE_ROW).getCell(3).value = 'ALVARADO, ZYRON JAY E.';
			writeX(august, MALE_ROW, dayColumns(august)[0]!);
		});
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();
		const august = diagnostic.months.find((month) => month.reportMonth === 'AUGUST');

		// Measured on the second file, and by name match rather than by the database's
		// row mappings - a row index from one workbook means nothing in another.
		expect(august?.sourceStatus).toBe('Comparable');
		expect(august?.rosterResolution).toBe('workbookNameMatch');
		expect(august?.workbookPath).toBe(LEGACY_WORKBOOK_PATH);
		expect(august?.rosterRows).toBe(1);
		expect(august?.counts.workbookXCount).toBe(1);
		// And the referenced file's own month keeps the database's row mappings.
		expect(septemberMonth(diagnostic).rosterResolution).toBe('databaseRowMappings');
	});

	it('finds the second file even though it has no extension', async () => {
		await septemberWorkbook(install.fixture);
		await addWorkbook(install.fixture, LEGACY_WORKBOOK_PATH, (workbook) => {
			hideSheet(workbook.worksheets[0]!);
		});
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();

		expect(diagnostic.workbooks.map((workbook) => workbook.path)).toEqual([
			install.fixture.path,
			LEGACY_WORKBOOK_PATH
		]);
		expect(diagnostic.workbooks[1]?.isReferencedByDatabase).toBe(false);
	});

	it('does not read a directory that is not there as a failure', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const diagnostic = await diagnoseSf2Marks();

		expect(diagnostic.workbooks).toHaveLength(1);
		expect(diagnostic.workbooks[0]?.readError).toBeUndefined();
	});
});

describe('a run that writes nothing', () => {
	it('leaves every row of every table exactly as it was', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);
		const before = await snapshotTables(install.driver);

		await diagnoseSf2Marks();

		expect(await snapshotTables(install.driver)).toEqual(before);
	});

	it('leaves the workbook bytes exactly as they were', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);
		const before = await install.fixture.fileSystem.readFile(install.fixture.path);

		await diagnoseSf2Marks();

		expect(Array.from(await install.fixture.fileSystem.readFile(install.fixture.path))).toEqual(
			Array.from(before)
		);
	});

	it('produces the same answer twice', async () => {
		await septemberWorkbook(install.fixture);
		await seedInstall(install.driver);

		const first = await diagnoseSf2Marks();
		const second = await diagnoseSf2Marks();

		// Rust ordered its cell sets through `HashSet`s, so a report could come out in a
		// different order on every run. `generatedAt` is the only field that may differ.
		expect({ ...second, generatedAt: 0 }).toEqual({ ...first, generatedAt: 0 });
	});
});

describe('the absence dates the SQL derives', () => {
	it('matches what the machine local time says, not the UTC day', async () => {
		await septemberWorkbook(install.fixture);
		// 22:30 on 31 August local is 14:30 UTC the same day in Manila, but an evening
		// this far east can land on the previous UTC day. Reading the UTC day would put
		// this absence in the wrong month.
		await seedInstall(install.driver, {
			absences: [{ id: 'e1', studentId: 's1', classId: CLASS_ID, date: '2026-09-01', hour: 23 }]
		});

		const diagnostic = await diagnoseSf2Marks();

		expect(diagnostic.eventCounts).toContainEqual({ eventType: 'absent', rows: 1 });
		expect(septemberMonth(diagnostic).counts.dbAbsentCount).toBe(1);
		expect(localDateOf('2026-09-01', 23)).toBe('2026-09-01');
	});
});

describe('the legacy roster rows the totals row must never join', () => {
	it('keeps a mark on the MALE TOTAL row out of the comparison', async () => {
		await septemberWorkbook(install.fixture, (_workbook, september) => {
			clearMarks(september);
			writeX(september, MALE_TOTAL_ROW, dayColumns(september)[0]!);
		});
		await seedInstall(install.driver, { absences: [] });

		const diagnostic = await diagnoseSf2Marks();
		const month = septemberMonth(diagnostic);

		// The workbook scan sees it - it is a real `X` on the sheet - but the roster has
		// no row 29, so it is out of scope and is neither a workbook mark nor an
		// unplaced sheet. Reporting it would be crying wolf about a subtotal row.
		expect(month.counts.workbookXCount).toBe(0);
		expect(month.cellsOnlyInWorkbook).toHaveLength(0);
		expect(diagnostic.unplacedSheets).toHaveLength(0);
		expect(diagnostic.workbooks[0]?.totalXCount).toBe(1);
	});
});
