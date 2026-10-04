import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { sf2MonthName } from '$lib/features/sf2/calendar';
import { reportYearForSchoolMonth } from '$lib/features/sf2/first-school-day';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import {
	createSf2MonthFile,
	getSf2LaunchMonth,
	getSf2MonthPreview,
	getSf2SchoolCalendarSettings,
	listSf2MonthWorkbooks,
	NO_SCHOOL_DAYS_MESSAGE,
	runSf2WorkbookSplit,
	setSf2SchoolStartDate,
	UNKNOWN_MONTH_MESSAGE,
	useMonthGridBuilder,
	type MonthGridBuilder
} from '../month';
import type { MonthSheetBuild, SchoolYearBuildReport } from '../workbook-builder';
import { replaceMonthRoster } from '../students';
import {
	EMPTY_DATE_ANALYSIS_MESSAGE,
	FIRST_SCHOOL_DAY_UNDETERMINED,
	replaceMonthDateMappings
} from '../templates';
import { db, insertMonthTemplate, useMonthTestDb } from './schema';
import type { Sf2ExportPreview, Sf2SplitOutcome } from '$lib/types';

/**
 * The two workbook writers are owned by the `workbook-builder` and `merge` ports.
 * This suite is about the month service, so they are mocked and the hand-off is
 * asserted on the arguments the service passes.
 */
const { buildSchoolYearWorkbook, mergeWorkbooks } = vi.hoisted(() => ({
	buildSchoolYearWorkbook:
		vi.fn<(path: string, builds: readonly MonthSheetBuild[]) => Promise<SchoolYearBuildReport>>(),
	mergeWorkbooks: vi.fn<() => Promise<Sf2SplitOutcome>>()
}));
vi.mock('../workbook-builder', async (importOriginal) => ({
	...(await importOriginal<typeof import('../workbook-builder')>()),
	buildSchoolYearWorkbook
}));
vi.mock('../merge', () => ({ mergeWorkbooks }));

const WORKBOOK = '/workbooks/SF2-GRADE-3-MATAPAT.xls';
const SCHOOL_YEAR = '2026-2027';

/** Today's calendar month and its school-year report year, from the real clock. */
const TODAY_MONTH = sf2MonthName(new Date().getMonth() + 1);
const TODAY_YEAR = reportYearForSchoolMonth(SCHOOL_YEAR, new Date().getMonth() + 1, 2026);

useMonthTestDb();

let fs: MemoryFileSystem;

const EMPTY_GRID: Sf2ExportPreview = {
	template: undefined,
	className: '',
	dates: [],
	students: [],
	absentList: [],
	mappedStudents: 0,
	mappedDates: 0,
	presentCount: 0,
	absenceCount: 0,
	unmappedStudentCount: 0,
	canExport: true,
	issues: [],
	warnings: []
};

/**
 * Echoes the month half back, so an assertion on `preview.dates` is an assertion
 * about the month read and not about the stub.
 */
const echoGrid: MonthGridBuilder = (input) => ({
	...EMPTY_GRID,
	dates: input.dates,
	mappedStudents: input.readiness.mappedStudents,
	mappedDates: input.readiness.mappedDates,
	canExport: input.readiness.canExport,
	issues: input.readiness.issues,
	warnings: input.readiness.warnings
});

const SPLIT_OUTCOME: Sf2SplitOutcome = {
	splitCompletedAt: 1234,
	months: [],
	verifiedCount: 12,
	needsAttentionCount: 0,
	workbookPath: WORKBOOK,
	legacyFilePath: '/workbooks/old.xls',
	legacyBackupPath: null,
	absencesOutsideSchoolYear: 0,
	message: 'All twelve months verified.'
};

beforeEach(async () => {
	fs = new MemoryFileSystem();
	useFileSystem(fs);
	await db().execute(
		`INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
		 VALUES ('class-1', 'Grade 3 - Matapat', '08:00', '15:00', '08:45', 1)`
	);
	await db().execute(
		`INSERT INTO settings (id, day_start, day_end, late_after, quarter, attendance_mode, school_year)
		 VALUES ('app', '08:00', '15:00', '08:45', '1st Quarter', 'manual', '${SCHOOL_YEAR}')`
	);
	// The grid half of a month read is `sf2::preview::export_preview`, owned by
	// the preview port. These tests are about the month half, so it is stubbed
	// and the hand-off is asserted directly.
	useMonthGridBuilder(echoGrid);
	mergeWorkbooks.mockResolvedValue(SPLIT_OUTCOME);
});

afterEach(() => {
	useMonthGridBuilder(null);
	useFileSystem(null);
	buildSchoolYearWorkbook.mockReset();
	mergeWorkbooks.mockReset();
});

async function seedLegacyTemplate(): Promise<void> {
	await db().execute(
		`INSERT INTO sf2_templates (id, source_path, active_class_id, imported_at, school_name)
		 VALUES ('legacy', ?, 'class-1', 1, 'Matapat Elementary')`,
		[WORKBOOK]
	);
}

describe('getSf2SchoolCalendarSettings', () => {
	test('reads both unset as null, which is an answer and not a failure', async () => {
		expect(await getSf2SchoolCalendarSettings()).toEqual({
			schoolStartDate: null,
			lastReportMonth: null
		});
	});

	test('a settings row that is not there is the same two nulls', async () => {
		await db().execute('DELETE FROM settings');
		expect((await getSf2SchoolCalendarSettings()).schoolStartDate).toBeNull();
	});
});

describe('setSf2SchoolStartDate', () => {
	test('records a real start date', async () => {
		await setSf2SchoolStartDate('2026-08-24');
		expect((await getSf2SchoolCalendarSettings()).schoolStartDate).toBe('2026-08-24');
	});

	test('null unsets it rather than defaulting to a guess', async () => {
		await setSf2SchoolStartDate('2026-08-24');
		await setSf2SchoolStartDate(null);
		expect((await getSf2SchoolCalendarSettings()).schoolStartDate).toBeNull();
	});

	test('refuses anything that is not YYYY-MM-DD', async () => {
		await expect(setSf2SchoolStartDate('08/24/2026')).rejects.toMatchObject({
			kind: 'InvalidInput'
		});
		expect((await getSf2SchoolCalendarSettings()).schoolStartDate).toBeNull();
	});

	test('a month the teacher dated by hand keeps that day', async () => {
		await insertMonthTemplate({ firstSchoolDay: FIRST_SCHOOL_DAY_UNDETERMINED });
		await setSf2SchoolStartDate('2026-08-24');
		// Re-derivation is a separate, explicit step; typing the start date must
		// never re-date a month the teacher already dated.
		expect((await getSf2MonthPreview('SEPTEMBER', 'class-1', SCHOOL_YEAR)).firstSchoolDay).toBe(
			FIRST_SCHOOL_DAY_UNDETERMINED
		);
	});
});

describe('listSf2MonthWorkbooks', () => {
	beforeEach(async () => {
		await fs.writeFileAtomic(WORKBOOK, 'x');
		await insertMonthTemplate({ id: 'm-sep', sourcePath: WORKBOOK });
	});

	test('always answers twelve rows, in school-year order', async () => {
		const rows = await listSf2MonthWorkbooks('class-1');
		expect(rows).toHaveLength(12);
		expect(rows[0].month).toBe('SEPTEMBER');
		expect(rows[11].month).toBe('AUGUST');
	});

	test('a month with no stored row still comes back, as "not set up yet"', async () => {
		const october = (await listSf2MonthWorkbooks('class-1')).find((r) => r.month === 'OCTOBER');
		expect(october?.hasTemplate).toBe(false);
		expect(october?.firstSchoolDay).toBe(FIRST_SCHOOL_DAY_UNDETERMINED);
		expect(october?.learnerCount).toBe(0);
	});

	test('every month of the class names the one workbook, so the list never lies', async () => {
		const rows = await listSf2MonthWorkbooks('class-1');
		expect(rows.every((r) => r.fileName === 'SF2-GRADE-3-MATAPAT.xls' && r.fileExists)).toBe(true);
	});

	test('a workbook that is gone reads as missing on all twelve rows', async () => {
		await fs.remove(WORKBOOK);
		expect((await listSf2MonthWorkbooks('class-1')).every((r) => !r.fileExists)).toBe(true);
	});

	test('falls back to the only class on record when none is named', async () => {
		expect((await listSf2MonthWorkbooks())[0].month).toBe('SEPTEMBER');
	});
});

describe('getSf2LaunchMonth', () => {
	beforeEach(async () => {
		await db().execute(`UPDATE settings SET last_report_month = 'MAY'`);
	});

	test('opens on today’s month when it has a workbook', async () => {
		await insertMonthTemplate({ reportMonth: TODAY_MONTH, reportYear: TODAY_YEAR });
		const launch = await getSf2LaunchMonth('class-1');
		expect(launch.month).toBe(TODAY_MONTH);
		expect(launch.reportYear).toBe(TODAY_YEAR);
		expect(launch.fellBack).toBe(false);
		expect(launch.todayMonth).toBe(TODAY_MONTH);
	});

	test('falls back to the last month used, and names both months', async () => {
		await setSf2SchoolStartDate('2026-08-24');
		await insertMonthTemplate({ reportMonth: 'MAY', reportYear: 2027 });
		const launch = await getSf2LaunchMonth('class-1');
		expect(launch.month).toBe('MAY');
		expect(launch.reportYear).toBe(2027);
		expect(launch.fellBack).toBe(true);
		expect(launch.todayMonth).toBe(TODAY_MONTH);
		// E1: the app is showing MAY, so the month to offer is today's.
		expect(launch.todayCanCreate).toBe(true);
		expect(launch.canCreate).toBe(false);
	});

	test('says so when there is nothing to fall back on', async () => {
		const launch = await getSf2LaunchMonth('class-1');
		expect(launch.fellBack).toBe(true);
		expect(launch.month).toBe(TODAY_MONTH);
		// One line for "no workbook for this month or the last one used". Months are
		// dated from June by default, so every month has school days.
		expect(launch.issues.join(' ')).toContain(`No SF2 workbook exists for ${TODAY_MONTH}`);
		expect(launch.issues).not.toContain(NO_SCHOOL_DAYS_MESSAGE);
	});

	test('dates months from June by default instead of asking for a start date', async () => {
		const launch = await getSf2LaunchMonth('class-1');
		expect(launch.needsSchoolStartDate).toBe(false);
		expect(launch.hasSchoolDays).toBe(true);
		expect(launch.canCreate).toBe(true);
	});
});

describe('getSf2MonthPreview', () => {
	test('rejects a month no name can resolve', async () => {
		await expect(getSf2MonthPreview('School Year', 'class-1')).rejects.toMatchObject({
			detail: UNKNOWN_MONTH_MESSAGE
		});
	});

	test('names the worksheet only for a month that has a row', async () => {
		await insertMonthTemplate({ id: 'm-sep', reportMonth: 'SEPTEMBER', reportYear: 2026 });
		expect((await getSf2MonthPreview('SEPTEMBER', 'class-1', SCHOOL_YEAR)).sheetName).toBe(
			'SEPTEMBER 2026'
		);
		expect((await getSf2MonthPreview('NOVEMBER', 'class-1', SCHOOL_YEAR)).sheetName).toBe('');
	});

	test('reads the stored year instead of recomputing it', async () => {
		await insertMonthTemplate({ reportMonth: 'AUGUST', reportYear: 2027 });
		expect((await getSf2MonthPreview('August', 'class-1', SCHOOL_YEAR)).reportYear).toBe(2027);
	});

	test('lists every Monday-Friday of the month, each on its own worksheet', async () => {
		const preview = await getSf2MonthPreview('SEPTEMBER', 'class-1', SCHOOL_YEAR);
		expect(preview.dates.map((d) => d.date)).toEqual([
			'2026-09-01',
			'2026-09-02',
			'2026-09-03',
			'2026-09-04',
			'2026-09-07',
			'2026-09-08',
			'2026-09-09',
			'2026-09-10',
			'2026-09-11',
			'2026-09-14',
			'2026-09-15',
			'2026-09-16',
			'2026-09-17',
			'2026-09-18',
			'2026-09-21',
			'2026-09-22',
			'2026-09-23',
			'2026-09-24',
			'2026-09-25',
			'2026-09-28',
			'2026-09-29',
			'2026-09-30'
		]);
		// A month with no row of its own has no worksheet to name.
		expect(preview.dates.every((d) => d.sheetName === '')).toBe(true);
	});

	test('a mapped day carries its own stored column and worksheet', async () => {
		await insertMonthTemplate({ id: 'm-sep', reportMonth: 'SEPTEMBER', reportYear: 2026 });
		await replaceMonthDateMappings('m-sep', [
			{
				templateId: 'm-sep',
				date: '2026-09-02',
				columnLetter: 'F',
				columnIndex: 6,
				sheetName: 'SEPTEMBER 2026'
			}
		]);
		const preview = await getSf2MonthPreview('SEPTEMBER', 'class-1', SCHOOL_YEAR);
		const mapped = preview.dates.filter((d) => d.columnLetter !== '');
		expect(mapped).toEqual([
			{ date: '2026-09-02', sheetName: 'SEPTEMBER 2026', columnLetter: 'F', columnIndex: 6 }
		]);
		expect(preview.gridEmpty).toBe(false);
	});

	test('serves a month from the pre-split tables and says where it drew from', async () => {
		await seedLegacyTemplate();
		await db().execute(
			`INSERT INTO sf2_date_mappings (template_id, sheet_name, date, column_letter, column_index)
			 VALUES ('legacy', 'SEPTEMBER 2026', '2026-09-01', 'E', 5),
			        ('legacy', 'OCTOBER 2026', '2026-10-01', 'E', 5)`
		);
		await db().execute(
			`INSERT INTO sf2_student_mappings (template_id, student_id, workbook_name, normalized_name, row_index)
			 VALUES ('legacy', 'stu-1', 'JUAN', 'JUAN', 8)`
		);

		const preview = await getSf2MonthPreview('SEPTEMBER', 'class-1', SCHOOL_YEAR);
		// September's columns and nothing else: the legacy table is keyed by a
		// full date with no month of its own, so the month is a closed range.
		expect(preview.dates.filter((d) => d.columnLetter !== '')).toHaveLength(1);
		expect(preview.dates.find((d) => d.date === '2026-09-01')?.columnLetter).toBe('E');
		expect(preview.usesLegacyMappings).toBe(true);
		expect(preview.warnings.join(' ')).toContain('before per-month workbooks existed');
		expect(preview.template?.schoolName).toBe('Matapat Elementary');
	});

	test('hands the grid builder the month roster and the day grid', async () => {
		await insertMonthTemplate({ id: 'm-sep', reportMonth: 'SEPTEMBER', reportYear: 2026 });
		await replaceMonthRoster('m-sep', [
			{
				templateId: 'm-sep',
				studentId: 'stu-1',
				workbookName: 'JUAN',
				normalizedName: 'JUAN',
				rowIndex: 8,
				genderBlock: 'MALE'
			}
		]);
		let rosterSeen = 0;
		let datesSeen = 0;
		let readinessSeen: number | undefined;
		const builder: MonthGridBuilder = (input) => {
			rosterSeen = input.roster.length;
			datesSeen = input.dates.length;
			readinessSeen = input.readiness.mappedStudents;
			return echoGrid(input);
		};
		useMonthGridBuilder(builder);

		await getSf2MonthPreview('SEPTEMBER', 'class-1', SCHOOL_YEAR);
		expect(rosterSeen).toBe(1);
		expect(datesSeen).toBe(22);
		expect(readinessSeen).toBe(1);
	});

	test('a class student with no learner row is named, not silently dropped', async () => {
		await db().execute(
			`INSERT INTO students (id, name, class_id, created_at) VALUES ('stu-1', 'JUAN', 'class-1', 1)`
		);
		await insertMonthTemplate({ reportMonth: 'SEPTEMBER', reportYear: 2026 });
		expect(
			(await getSf2MonthPreview('SEPTEMBER', 'class-1', SCHOOL_YEAR)).issues.join(' ')
		).toContain('1 of the class');
	});
});

describe('createSf2MonthFile', () => {
	beforeEach(async () => {
		await seedLegacyTemplate();
		await setSf2SchoolStartDate('2026-08-24');
		await fs.writeFileAtomic(WORKBOOK, 'x');
	});

	test('refuses a month with no school days, before it touches anything', async () => {
		// Classes start on 5 October, so SEPTEMBER 2026 — the first month of the
		// 2026-2027 school year — is over before they do and has no day to record.
		await setSf2SchoolStartDate('2026-10-05');
		await expect(createSf2MonthFile('SEPTEMBER', 'class-1')).rejects.toMatchObject({
			detail: NO_SCHOOL_DAYS_MESSAGE
		});
	});

	test('refuses a month that is already on record', async () => {
		await insertMonthTemplate({ reportMonth: 'SEPTEMBER', reportYear: 2026 });
		await expect(createSf2MonthFile('SEPTEMBER', 'class-1')).rejects.toMatchObject({
			kind: 'InvalidInput'
		});
	});

	test('refuses when the class has no workbook at all', async () => {
		await db().execute('DELETE FROM sf2_templates');
		await expect(createSf2MonthFile('SEPTEMBER', 'class-1')).rejects.toMatchObject({
			kind: 'InvalidInput'
		});
	});

	test('refuses when the workbook on record is not on disk', async () => {
		await fs.remove(WORKBOOK);
		await expect(createSf2MonthFile('SEPTEMBER', 'class-1')).rejects.toMatchObject({
			kind: 'InvalidInput'
		});
	});

	test('a build that does not verify writes no month row', async () => {
		buildSchoolYearWorkbook.mockResolvedValue({
			months: [
				{
					sheetName: 'SEPTEMBER 2026',
					dates: [],
					writtenMarks: 0,
					extraRosterRows: 0,
					unmappedAbsences: { students: 0, dates: 0 },
					verification: {
						verified: false,
						expectedX: 4,
						foundX: 0,
						expectedLearners: 2,
						foundLearners: 0
					}
				}
			],
			removedSheets: [],
			keptHelperSheets: [],
			verification: { verified: true }
		});

		await expect(createSf2MonthFile('SEPTEMBER', 'class-1')).rejects.toMatchObject({
			kind: 'Internal'
		});
		expect(
			(await listSf2MonthWorkbooks('class-1')).find((r) => r.month === 'SEPTEMBER')
		).toMatchObject({ hasTemplate: false });
	});

	test('a verified build records the month, its grid and the marks it counted', async () => {
		buildSchoolYearWorkbook.mockResolvedValue({
			months: [
				{
					sheetName: 'SEPTEMBER 2026',
					dates: [
						{
							templateId: 'unused',
							date: '2026-09-01',
							columnLetter: 'E',
							columnIndex: 5,
							sheetName: 'SEPTEMBER 2026'
						}
					],
					writtenMarks: 4,
					extraRosterRows: 0,
					unmappedAbsences: { students: 0, dates: 0 },
					verification: { verified: true }
				}
			],
			removedSheets: [],
			keptHelperSheets: [],
			verification: { verified: true }
		});

		const created = await createSf2MonthFile('SEPTEMBER', 'class-1');
		expect(created.reportMonth).toBe('SEPTEMBER');
		expect(created.reportYear).toBe(2026);
		// The build counted the marks it just wrote, so the month is measured
		// rather than "never scanned".
		expect(created.workbookXCount).toBe(4);
		expect(created.workbookScannedAt).not.toBeNull();
		expect(created.firstSchoolDay).toBe(1);

		const [path, builds] = buildSchoolYearWorkbook.mock.calls[0];
		expect(path).toBe(WORKBOOK);
		// `removeStaleSheets: false` is the additive path: the other eleven
		// worksheets are the teacher's, not this call's to rewrite.
		expect(builds).toHaveLength(1);
		expect(builds[0].removeStaleSheets).toBe(false);
		expect(builds[0].request.firstSchoolDay).toBe(1);

		const preview = await getSf2MonthPreview('SEPTEMBER', 'class-1', SCHOOL_YEAR);
		expect(preview.hasTemplate).toBe(true);
		expect(preview.dates.filter((d) => d.columnLetter !== '')).toHaveLength(1);
	});
});

describe('runSf2WorkbookSplit', () => {
	test('hands the job to the merge and returns its outcome unchanged', async () => {
		expect(await runSf2WorkbookSplit()).toEqual(SPLIT_OUTCOME);
		expect(mergeWorkbooks).toHaveBeenCalledOnce();
	});
});

describe('the guards on a month grid', () => {
	test('an empty day grid is refused rather than committed', async () => {
		// Committing it would delete every day column for the month, which
		// leaves the reports grid empty and the next write with nothing to write.
		await insertMonthTemplate({});
		await expect(replaceMonthDateMappings('month-1', [])).rejects.toMatchObject({
			detail: EMPTY_DATE_ANALYSIS_MESSAGE
		});
	});
});
