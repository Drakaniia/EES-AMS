/**
 * The calendar header layout, against the real bundled template.
 *
 * These assertions are against `TEMPLATE_AUTOMATED_SF2.xlsx` — Excel's own
 * conversion of the original DepEd `.xls` — so the slot list, the merged column
 * pairs and the shipped day numbers are ground truth rather than something this
 * suite invented. No Excel install is involved.
 *
 * The strongest assertion here is {@link dayNumbersForSlots} versus the template's
 * own JULY 2025 header: the arithmetic has to reproduce the grid Excel itself drew,
 * column for column, including the empty Monday of a month whose first school day
 * is a Tuesday.
 */
import { describe, expect, it } from 'vitest';
import type { Worksheet } from 'exceljs';
import { loadTemplate } from '$lib/features/excel/__tests__/template-fixture';
import {
	columnLetter,
	columnNumber,
	getCellText,
	monthNumber,
	setCellText,
	writableDayColumns
} from '$lib/features/excel/workbook';
import {
	bestSf2MonthlySheet,
	configureSf2Calendar,
	dayNumbersForSlots,
	daysWithoutASlot,
	parseWeekdayLabel,
	sf2WeekdaySlots,
	validateConfiguredCalendar
} from '../calendar';
import type { Sf2DaySlot, Sf2TemplateMetadata, Sf2WorkbookAnalysis } from '../calendar';

function appErrorOf(run: () => unknown): { kind: string; detail: string } {
	try {
		run();
	} catch (thrown) {
		return thrown as { kind: string; detail: string };
	}
	throw new Error('expected an AppError, nothing was thrown');
}

/** The first Monday-Friday of a month, which is the only day a grid may start on. */
function firstSchoolDayOf(year: number, month: number): number {
	for (let day = 1; day <= 31; day += 1) {
		const date = new Date(Date.UTC(year, month - 1, day));
		if (date.getUTCMonth() + 1 !== month) break;
		const weekday = date.getUTCDay();
		if (weekday >= 1 && weekday <= 5) return day;
	}
	throw new Error(`${month}/${year} has no school day`);
}

/** The day numbers a sheet shows across its writable day columns. */
function headerDays(sheet: Worksheet): string[] {
	return writableDayColumns(sheet).map((letter) => getCellText(sheet, 6, columnNumber(letter)));
}

/** `headerDays`, as the non-empty day numbers only - the columns a month fills. */
function filledDays(sheet: Worksheet): string[] {
	return headerDays(sheet).filter((day) => day !== '');
}

function metadataFor(overrides: Partial<Sf2TemplateMetadata> = {}): Sf2TemplateMetadata {
	return {
		schoolId: '401234',
		schoolName: 'TAPATIN ELEMENTARY SCHOOL',
		schoolYear: '2025-2026',
		reportMonth: 'JULY',
		gradeLevel: '3',
		section: 'MATAPAT',
		adviserName: '',
		schoolHeadName: '',
		configureCalendar: true,
		firstSchoolDay: 1,
		...overrides
	};
}

describe('the weekday header of the real template', () => {
	it('is five weeks of Monday..Friday, one slot per merge master', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet('JUNE 2025')!;
		const slots = sf2WeekdaySlots(sheet);

		expect(slots.map((slot) => columnLetter(slot.column))).toEqual([
			'F',
			'H',
			'I',
			'J',
			'K',
			'L',
			'N',
			'O',
			'P',
			'Q',
			'R',
			'T',
			'U',
			'V',
			'X',
			'Z',
			'AB',
			'AC',
			'AD',
			'AE',
			'AF',
			'AG',
			'AI',
			'AJ',
			'AK'
		]);
		expect(slots.map((slot) => slot.weekIndex)).toEqual([
			0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4
		]);
		expect(slots.map((slot) => slot.weekdayIndex)).toEqual([
			0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4
		]);
	});

	it('agrees with the writable day columns the template exposes', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet('JUNE 2025')!;
		expect(sf2WeekdaySlots(sheet).map((slot) => columnLetter(slot.column))).toEqual(
			writableDayColumns(sheet)
		);
	});

	it('reads only the first half of a merged pair, never the ABSENT column', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet('JUNE 2025')!;
		const columns = sf2WeekdaySlots(sheet).map((slot) => slot.column);
		expect(columns).not.toContain(7);
		expect(Math.max(...columns)).toBeLessThan(39);
	});

	it('skips a non-weekday label', () => {
		expect(parseWeekdayLabel('ABSENT')).toBeUndefined();
		expect(parseWeekdayLabel('')).toBeUndefined();
		expect(parseWeekdayLabel('Mon')).toBe(0);
		expect(parseWeekdayLabel('T')).toBe(1);
		expect(parseWeekdayLabel('TH')).toBe(3);
		expect(parseWeekdayLabel('Tuesday')).toBe(1);
		expect(parseWeekdayLabel('Friday')).toBe(4);
	});
});

describe('which day goes in which column', () => {
	it('reproduces the grid Excel itself drew for July 2025', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet('JULY 2025')!;

		// 1 July 2025 is a Tuesday, so the week before it holds a June Monday that
		// this month cannot use: the first slot is empty and the rest are not.
		expect(dayNumbersForSlots(2025, 7, 1, sf2WeekdaySlots(sheet))).toEqual([
			{ column: 6 },
			{ column: 8, day: 1 },
			{ column: 9, day: 2 },
			{ column: 10, day: 3 },
			{ column: 11, day: 4 },
			{ column: 12, day: 7 },
			{ column: 14, day: 8 },
			{ column: 15, day: 9 },
			{ column: 16, day: 10 },
			{ column: 17, day: 11 },
			{ column: 18, day: 14 },
			{ column: 20, day: 15 },
			{ column: 21, day: 16 },
			{ column: 22, day: 17 },
			{ column: 24, day: 18 },
			{ column: 26, day: 21 },
			{ column: 28, day: 22 },
			{ column: 29, day: 23 },
			{ column: 30, day: 24 },
			{ column: 31, day: 25 },
			{ column: 32, day: 28 },
			{ column: 33, day: 29 },
			{ column: 35, day: 30 },
			{ column: 36, day: 31 },
			{ column: 37 }
		]);
	});

	it('lays June 2025 out on the 21 columns the template fills', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet('JUNE 2025')!;

		// 2 June 2025 is a Monday, so the month opens on the first slot and only its
		// last Monday - the 30th - reaches the fifth week.
		expect(filledDays(sheet)).toEqual([
			'2',
			'3',
			'4',
			'5',
			'6',
			'9',
			'10',
			'11',
			'12',
			'13',
			'16',
			'17',
			'18',
			'19',
			'20',
			'23',
			'24',
			'25',
			'26',
			'27',
			'30'
		]);
		expect(writableDayColumns(sheet).length).toBe(25);
	});

	it('starts a late-starting class on its start day, not on the 1st', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet('JUNE 2025')!;
		const slots = sf2WeekdaySlots(sheet);

		// 9 June 2025 is a Monday, so the grid opens on the first slot and reaches
		// three full weeks plus the 30th.
		const layout = dayNumbersForSlots(2025, 6, 9, slots);
		expect(layout.find((entry) => entry.column === 6)).toEqual({ column: 6, day: 9 });
		expect(layout.filter((entry) => entry.day !== undefined)).toHaveLength(16);
	});

	it('drops no school day of any real month', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet('JUNE 2025')!;
		const slots = sf2WeekdaySlots(sheet);

		// A 31-day month opening on a Monday is the worst case: 23 school days into
		// 25 slots. Every month from 2024 to 2027 is checked rather than reasoned
		// about, and each is anchored on its own first Monday-Friday - a grid anchored
		// on a weekend day is a grid nothing ever asks for.
		for (let year = 2024; year <= 2027; year += 1) {
			for (let month = 1; month <= 12; month += 1) {
				const firstDay = firstSchoolDayOf(year, month);
				expect(daysWithoutASlot(year, month, firstDay, slots)).toEqual([]);
				expect(dayNumbersForSlots(year, month, firstDay, slots).filter((e) => e.day)).not.toEqual(
					[]
				);
			}
		}
	});

	it('reports the school days a short grid cannot hold', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet('JUNE 2025')!;
		// Drop the fifth week: July 2025 then has four days with nowhere to go.
		const truncated: Sf2DaySlot[] = sf2WeekdaySlots(sheet).slice(0, 20);

		expect(daysWithoutASlot(2025, 7, 1, truncated)).toEqual([28, 29, 30, 31]);
	});
});

describe('configureSf2Calendar', () => {
	it('writes the header of the month it was asked for', async () => {
		const fixture = await loadTemplate();
		const workbook = await fixture.open();

		const result = configureSf2Calendar(workbook, metadataFor({ firstSchoolDay: 7 }));

		expect(result.sheetName).toBe('JULY 2025');
		expect(result.droppedSchoolDays).toEqual([]);
		const sheet = workbook.getWorksheet('JULY 2025')!;
		expect(filledDays(sheet)).toEqual([
			'7',
			'8',
			'9',
			'10',
			'11',
			'14',
			'15',
			'16',
			'17',
			'18',
			'21',
			'22',
			'23',
			'24',
			'25',
			'28',
			'29',
			'30',
			'31'
		]);
	});

	it('clears a column the new month has no school day for', async () => {
		const fixture = await loadTemplate();
		const workbook = await fixture.open();
		const sheet = workbook.getWorksheet('JUNE 2025')!;

		// Stale numbers from a longer month, as a sheet carried over from one report
		// month to the next would hold. A day number that survives is read as a
		// school day that never happened.
		for (const letter of writableDayColumns(sheet)) {
			setCellText(sheet, 6, columnNumber(letter), '31');
		}

		const result = configureSf2Calendar(
			workbook,
			metadataFor({ reportMonth: 'JUNE', firstSchoolDay: 2 })
		);

		expect(result.sheetName).toBe('JUNE 2025');
		expect(filledDays(sheet)).toEqual([
			'2',
			'3',
			'4',
			'5',
			'6',
			'9',
			'10',
			'11',
			'12',
			'13',
			'16',
			'17',
			'18',
			'19',
			'20',
			'23',
			'24',
			'25',
			'26',
			'27',
			'30'
		]);
	});

	it('renames a sheet the workbook has no month tab for, and lands on it', async () => {
		const fixture = await loadTemplate();
		const workbook = await fixture.open();

		const result = configureSf2Calendar(
			workbook,
			metadataFor({ reportMonth: 'NOVEMBER', firstSchoolDay: 4 })
		);

		// November 2025 opens on a Saturday, so the first column that can hold a day
		// is the Tuesday the 4th.
		expect(result.sheetName).toBe('NOVEMBER 2025');
		expect(filledDays(workbook.getWorksheet('NOVEMBER 2025')!)[0]).toBe('4');
		expect(workbook.views[0]?.activeTab).toBe(
			workbook.worksheets.findIndex((sheet) => sheet.name === 'NOVEMBER 2025')
		);
	});

	it('refuses a first attendance day the month cannot hold', async () => {
		const fixture = await loadTemplate();
		const workbook = await fixture.open();
		expect(() =>
			configureSf2Calendar(workbook, metadataFor({ reportMonth: 'JUNE', firstSchoolDay: 31 }))
		).toThrow();
	});

	it('refuses a weekend first attendance day', async () => {
		const fixture = await loadTemplate();
		const workbook = await fixture.open();
		expect(
			appErrorOf(() =>
				configureSf2Calendar(workbook, metadataFor({ reportMonth: 'JULY', firstSchoolDay: 5 }))
			)
		).toEqual({
			kind: 'InvalidInput',
			detail: 'First attendance day must be a Monday-Friday school day'
		});
	});

	it('refuses a report month that names no month', async () => {
		const fixture = await loadTemplate();
		const workbook = await fixture.open();
		expect(
			appErrorOf(() => configureSf2Calendar(workbook, metadataFor({ reportMonth: 'smiling' })))
		).toEqual({
			kind: 'InvalidInput',
			detail: 'Report Month must be a valid month name'
		});
	});
});

describe('choosing the sheet to write', () => {
	it('picks the most populated monthly sheet', async () => {
		const fixture = await loadTemplate();
		const workbook = await fixture.open();
		const best = bestSf2MonthlySheet(workbook.worksheets.filter((s) => monthNumber(s.name) > 0));

		expect(best).toBeDefined();
		// The bundled template ships a 12-boy, 14-girl roster on its month sheets.
		expect(best!.name).toMatch(/^(JUNE|JULY|AUGUST|SEPT\.|OCTOBER) 2025$/);
	});
});

describe('validateConfiguredCalendar', () => {
	const july = (dates: string[]): Sf2WorkbookAnalysis =>
		({
			reportMonth: 'JULY',
			schoolYear: '2025-2026',
			dates: dates.map((date) => ({ sheetName: 'JULY 2025', date }))
		}) as Sf2WorkbookAnalysis;

	it('says nothing about a template whose calendar it will not write', () => {
		expect(() =>
			validateConfiguredCalendar(july(['2025-07-07']), metadataFor({ configureCalendar: false }))
		).not.toThrow();
	});

	it('accepts a grid that starts on the day the metadata claims', () => {
		expect(() =>
			validateConfiguredCalendar(july(['2025-07-01', '2025-07-02']), metadataFor())
		).not.toThrow();
	});

	it('names both days when the grid starts somewhere else', () => {
		expect(
			appErrorOf(() => validateConfiguredCalendar(july(['2025-07-07']), metadataFor())).detail
		).toBe(
			'SF2 calendar was not configured correctly: expected first attendance day 1, but the workbook starts at day 7'
		);
	});

	it('reports a grid with no dates at all', () => {
		expect(appErrorOf(() => validateConfiguredCalendar(july([]), metadataFor())).detail).toBe(
			'SF2 calendar was not configured correctly: no attendance dates were detected'
		);
	});

	it('requires a first day before it can judge one', () => {
		expect(
			appErrorOf(() =>
				validateConfiguredCalendar(july(['2025-07-01']), metadataFor({ firstSchoolDay: undefined }))
			).detail
		).toBe('First attendance day is required for SF2 templates');
	});
});
