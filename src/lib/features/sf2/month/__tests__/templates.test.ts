import { beforeEach, describe, expect, test } from 'vitest';
import { sf2MonthNumber } from '$lib/features/sf2/calendar';
import {
	clearFirstSchoolDayOverride,
	clearLastSyncedAtForClass,
	deriveMonthFirstSchoolDay,
	deleteMonthTemplate,
	findMonthTemplate,
	findMonthTemplateById,
	listAllMonthTemplates,
	listMonthDateMappings,
	listMonthTemplatesForSchoolYear,
	overrideMonthFirstSchoolDay,
	recordMonthWorkbookXCount,
	resolvedSheetName,
	replaceMonthDateMappings,
	setMonthLastSyncedAt,
	sheetNameFromDate,
	upsertMonthTemplate,
	updateMonthTemplate,
	FIRST_SCHOOL_DAY_UNDETERMINED
} from '../templates';
import type { Sf2MonthTemplate } from '$lib/types';
import { db, insertMonthTemplate, useMonthTestDb } from './schema';

useMonthTestDb();

describe('sf2MonthNumber (the month-name helper this repo sorts by)', () => {
	test('resolves the abbreviated forms the DepEd form prints', () => {
		expect(sf2MonthNumber('Sept.')).toBe(9);
		expect(sf2MonthNumber(' sept ')).toBe(9);
		expect(sf2MonthNumber('SEPTEMBER')).toBe(9);
	});

	test('is undefined for anything that names no month', () => {
		expect(sf2MonthNumber('School Year')).toBeUndefined();
	});
});

describe('sheetNameFromDate', () => {
	test('names the worksheet a date implies', () => {
		expect(sheetNameFromDate('2026-09-14')).toBe('SEPTEMBER 2026');
	});

	test('is undefined for text that is not an ISO date', () => {
		expect(sheetNameFromDate('14/09/2026')).toBeUndefined();
		expect(sheetNameFromDate('2026-13-01')).toBeUndefined();
	});
});

describe('resolvedSheetName', () => {
	test('prefers the stored column over the derivation', () => {
		expect(
			resolvedSheetName({
				templateId: 'm',
				date: '2026-09-14',
				columnLetter: 'F',
				columnIndex: 6,
				sheetName: 'OCTOBER 2026'
			})
		).toBe('OCTOBER 2026');
	});

	test('derives the same string when the column is absent or blank', () => {
		expect(
			resolvedSheetName({ templateId: 'm', date: '2026-09-14', columnLetter: 'F', columnIndex: 6 })
		).toBe('SEPTEMBER 2026');
		expect(
			resolvedSheetName({
				templateId: 'm',
				date: '2026-09-14',
				columnLetter: 'F',
				columnIndex: 6,
				sheetName: '  '
			})
		).toBe('SEPTEMBER 2026');
	});
});

describe('findMonthTemplate', () => {
	test('finds the row a month switch asks for', async () => {
		await insertMonthTemplate({});
		const found = await findMonthTemplate('class-1', '2026-2027', 'SEPTEMBER');
		expect(found?.id).toBe('month-1');
		expect(found?.reportYear).toBe(2026);
	});

	test('normalises the school year on the way in, so a spaced label still finds the row', async () => {
		await insertMonthTemplate({ schoolYear: '2026-2027' });
		expect(await findMonthTemplate('class-1', '2026 - 2027', 'SEPTEMBER')).toBeDefined();
	});

	test('normalises the school year on read as well as on write', async () => {
		// A row stored before v23. v23 backfills stored labels; normalising only
		// the write side would leave two formats in one database for every future
		// reader to trip over, so the reader normalises too.
		await db().execute(
			`INSERT INTO sf2_month_templates
			 (id, active_class_id, school_year, report_month, report_year, source_path, source_hash, first_school_day, imported_at)
			 VALUES ('old', 'class-1', '2026 - 2027', 'SEPTEMBER', 2026, 'p', 'h', 1, 1)`
		);
		expect((await findMonthTemplateById('old'))?.schoolYear).toBe('2026-2027');
	});

	test('is undefined for a month with no row', async () => {
		expect(await findMonthTemplate('class-1', '2026-2027', 'OCTOBER')).toBeUndefined();
	});
});

describe('listMonthTemplatesForSchoolYear', () => {
	test('reads SEPTEMBER -> AUGUST rather than alphabetically', async () => {
		await insertMonthTemplate({ id: 'jan', reportMonth: 'JANUARY', reportYear: 2027 });
		await insertMonthTemplate({ id: 'aug', reportMonth: 'AUGUST', reportYear: 2027 });
		await insertMonthTemplate({ id: 'sep', reportMonth: 'SEPTEMBER', reportYear: 2026 });

		const months = await listMonthTemplatesForSchoolYear('class-1', '2026-2027');
		expect(months.map((m) => m.id)).toEqual(['sep', 'jan', 'aug']);
	});
});

describe('listAllMonthTemplates', () => {
	test('puts the newest school year first, then school-year order', async () => {
		await insertMonthTemplate({
			id: 'old-aug',
			schoolYear: '2015-2016',
			reportMonth: 'AUGUST',
			reportYear: 2016
		});
		await insertMonthTemplate({
			id: 'new-jan',
			schoolYear: '2026-2027',
			reportMonth: 'JANUARY',
			reportYear: 2027
		});
		await insertMonthTemplate({
			id: 'new-sep',
			schoolYear: '2026-2027',
			reportMonth: 'SEPTEMBER',
			reportYear: 2026
		});

		const months = await listAllMonthTemplates();
		expect(months.map((m) => m.id)).toEqual(['new-sep', 'new-jan', 'old-aug']);
	});
});

describe('upsertMonthTemplate', () => {
	const template = (over: Partial<Sf2MonthTemplate> = {}): Sf2MonthTemplate => ({
		id: 'month-1',
		classId: 'class-1',
		schoolYear: '2026-2027',
		reportMonth: 'SEPTEMBER',
		reportYear: 2026,
		sourcePath: 'C:/workbooks/SF2.xls',
		sourceHash: 'hash-1',
		firstSchoolDay: 3,
		importedAt: 100,
		workbookXCount: 0,
		...over
	});

	test('creates the row', async () => {
		await upsertMonthTemplate(template({ schoolName: 'Matapat' }));
		const found = await findMonthTemplateById('month-1');
		expect(found?.schoolName).toBe('Matapat');
		expect(found?.firstSchoolDay).toBe(3);
	});

	test('refreshes metadata without moving a typed day or forgetting a sync', async () => {
		await upsertMonthTemplate(template());
		await overrideMonthFirstSchoolDay('month-1', 8);
		await setMonthLastSyncedAt('month-1', 555);

		await upsertMonthTemplate(
			template({ firstSchoolDay: 3, sourcePath: 'C:/other.xls', lastSyncedAt: undefined })
		);

		const found = await findMonthTemplateById('month-1');
		expect(found?.sourcePath).toBe('C:/other.xls');
		expect(found?.firstSchoolDay).toBe(8);
		expect(found?.lastSyncedAt).toBe(555);
	});

	test('stores the school year in its canonical form so the conflict target works', async () => {
		await upsertMonthTemplate(template({ schoolYear: '2026 - 2027' }));
		await upsertMonthTemplate(template({ id: 'second', schoolYear: '2026-2027' }));
		const all = await listMonthTemplatesForSchoolYear('class-1', '2026-2027');
		expect(all).toHaveLength(1);
	});
});

describe('updateMonthTemplate', () => {
	test('refreshes a row that is there', async () => {
		await upsertMonthTemplate({
			id: 'month-1',
			classId: 'class-1',
			schoolYear: '2026-2027',
			reportMonth: 'SEPTEMBER',
			reportYear: 2026,
			sourcePath: 'a',
			sourceHash: 'h',
			firstSchoolDay: 1,
			importedAt: 1,
			workbookXCount: 0
		});
		await updateMonthTemplate({
			id: 'month-1',
			classId: 'class-1',
			schoolYear: '2026-2027',
			reportMonth: 'SEPTEMBER',
			reportYear: 2026,
			sourcePath: 'b',
			sourceHash: 'h2',
			section: '3B',
			firstSchoolDay: 1,
			importedAt: 2,
			workbookXCount: 0
		});
		const found = await findMonthTemplateById('month-1');
		expect(found?.sourcePath).toBe('b');
		expect(found?.section).toBe('3B');
	});

	test('rejects a row that is not there', async () => {
		await expect(
			updateMonthTemplate({
				id: 'missing',
				classId: 'class-1',
				schoolYear: '2026-2027',
				reportMonth: 'SEPTEMBER',
				reportYear: 2026,
				sourcePath: 'a',
				sourceHash: 'h',
				firstSchoolDay: 1,
				importedAt: 1,
				workbookXCount: 0
			})
		).rejects.toMatchObject({ kind: 'InvalidInput' });
	});
});

describe('first school day', () => {
	beforeEachTemplate();

	test('an override moves the effective day and records its provenance', async () => {
		expect(await overrideMonthFirstSchoolDay('month-1', 8)).toBe(true);
		const found = await findMonthTemplateById('month-1');
		expect(found?.firstSchoolDay).toBe(8);
		expect(found?.firstSchoolDayOverride).toBe(8);
	});

	test('a derived day never overwrites an override', async () => {
		await overrideMonthFirstSchoolDay('month-1', 8);
		expect(await deriveMonthFirstSchoolDay('month-1', 3)).toBe(false);
		expect((await findMonthTemplateById('month-1'))?.firstSchoolDay).toBe(8);
	});

	test('a derived day never writes the undetermined sentinel over a real day', async () => {
		expect(await deriveMonthFirstSchoolDay('month-1', 0)).toBe(false);
		expect((await findMonthTemplateById('month-1'))?.firstSchoolDay).toBe(1);
	});

	test('clearing an override goes back to the derived day', async () => {
		await overrideMonthFirstSchoolDay('month-1', 8);
		expect(await clearFirstSchoolDayOverride('month-1', 3)).toBe(true);
		const found = await findMonthTemplateById('month-1');
		expect(found?.firstSchoolDay).toBe(3);
		expect(found?.firstSchoolDayOverride).toBeUndefined();
	});

	test('clearing an override may leave the month undated', async () => {
		await overrideMonthFirstSchoolDay('month-1', 8);
		await clearFirstSchoolDayOverride('month-1', FIRST_SCHOOL_DAY_UNDETERMINED);
		expect((await findMonthTemplateById('month-1'))?.firstSchoolDay).toBe(0);
	});
});

describe('sync and measurement state', () => {
	beforeEachTemplate();

	test('the X count is stored with the time it was measured', async () => {
		expect(await recordMonthWorkbookXCount('month-1', 37, 999)).toBe(true);
		const found = await findMonthTemplateById('month-1');
		expect(found?.workbookXCount).toBe(37);
		expect(found?.workbookScannedAt).toBe(999);
	});

	test('a grid correction marks every month of the class stale, not one', async () => {
		await insertMonthTemplate({ id: 'month-2', reportMonth: 'OCTOBER' });
		await setMonthLastSyncedAt('month-1', 111);
		await setMonthLastSyncedAt('month-2', 222);

		expect(await clearLastSyncedAtForClass('class-1')).toBe(2);
		expect((await findMonthTemplateById('month-1'))?.lastSyncedAt).toBeUndefined();
		expect((await findMonthTemplateById('month-2'))?.lastSyncedAt).toBeUndefined();
	});
});

describe('deleteMonthTemplate', () => {
	test('reports whether a row went', async () => {
		await insertMonthTemplate({});
		expect(await deleteMonthTemplate('month-1')).toBe(true);
		expect(await deleteMonthTemplate('month-1')).toBe(false);
	});
});

describe('the day grid', () => {
	test('is replaced in one go and read back in date order', async () => {
		await replaceMonthDateMappings('month-1', [
			{
				templateId: 'month-1',
				date: '2026-09-02',
				columnLetter: 'F',
				columnIndex: 6,
				sheetName: 'SEPTEMBER 2026'
			},
			{
				templateId: 'month-1',
				date: '2026-09-01',
				columnLetter: 'E',
				columnIndex: 5,
				sheetName: 'SEPTEMBER 2026'
			}
		]);
		const grid = await listMonthDateMappings('month-1');
		expect(grid.map((d) => d.date)).toEqual(['2026-09-01', '2026-09-02']);
	});

	test('is stored under the month it was given, not the id the records carry', async () => {
		// The build tags its dates with the id it guessed; the row it lands on may be
		// a different one. A grid filed under the guess is a grid no month can read.
		await replaceMonthDateMappings('month-1', [
			{ templateId: 'provisional', date: '2026-09-01', columnLetter: 'F', columnIndex: 6 }
		]);
		expect(await listMonthDateMappings('month-1')).toHaveLength(1);
		expect(await listMonthDateMappings('provisional')).toHaveLength(0);
	});

	test('a replace never reaches another month', async () => {
		await insertMonthTemplate({ id: 'month-2', reportMonth: 'OCTOBER' });
		await db().execute(
			`INSERT INTO sf2_month_date_mappings (template_id, date, column_letter, column_index)
			 VALUES ('month-2', '2026-10-01', 'E', 5)`
		);
		await replaceMonthDateMappings('month-1', [
			{ templateId: 'month-1', date: '2026-09-01', columnLetter: 'F', columnIndex: 6 }
		]);
		expect(await listMonthDateMappings('month-2')).toHaveLength(1);
	});
});

function beforeEachTemplate(): void {
	beforeEach(async () => {
		await insertMonthTemplate({});
	});
}
