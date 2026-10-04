/**
 * The geometry behind building a month sheet, and the build itself against the
 * real converted DepEd template.
 *
 * Nothing here needs Excel installed: the template is read as bytes from
 * `src-tauri/resources/sf2/TEMPLATE_AUTOMATED_SF2.xlsx` (never written to) and
 * every build runs against a `MemoryFileSystem`. What is tested is the arithmetic
 * that decides where a day goes, which cells get written, and whether a saved file
 * still holds what the build said it would - the three things that silently corrupt
 * a workbook when they are wrong.
 */

import type { Worksheet } from 'exceljs';
import { beforeEach, describe, expect, it } from 'vitest';
import {
	loadTemplate,
	TEMPLATE_ROSTER,
	TEMPLATE_SHEETS,
	TEMPLATE_TOTALS,
	type TemplateFixture
} from '$lib/features/excel/__tests__/template-fixture';
import {
	formulaOf,
	formulaResult,
	getCellText,
	writableDayColumns
} from '$lib/features/excel/workbook';
import { readLearnerRows } from '$lib/features/excel/roster';
import type { Sf2LearnerRow } from '$lib/features/excel/types';
import { weekdaySlots, type MonthDaySlot } from '../workbook-sheets';
import {
	attendanceBands,
	attendanceBandsForWrites,
	buildSchoolYearWorkbook,
	combineVerifications,
	dateInColumn,
	dayNumbersForSlots,
	daysWithoutASlot,
	headerRowShift,
	monthDateMappings,
	readLegacyMonths,
	resolveMarks,
	rosterExpansionFor,
	sheetNamesOf,
	sourceFemaleStartRow,
	verifyMonthBuild,
	type MonthBuildRequest,
	type MonthLearnerWrite,
	type RowBand
} from '../workbook-builder';

/**
 * The 25 day slots of the bundled template: five weeks of Monday..Friday, in the
 * columns its weekday header actually labels.
 *
 * Columns 7, 13, 19, 23, 25, 27, 34 and 38 are the second halves of merged pairs
 * and carry no label, so they are not slots.
 */
const LABELLED_COLUMNS = [
	6, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 20, 21, 22, 24, 26, 28, 29, 30, 31, 32, 33, 35, 36, 37
];

const BUNDLED_SLOTS: MonthDaySlot[] = LABELLED_COLUMNS.map((column, index) => ({
	column,
	weekIndex: Math.floor(index / 5),
	weekdayIndex: index % 5
}));

/** Monday-to-Friday school days of a month, as `{ month, day }` pairs. */
function schoolDays(year: number, month: number): number[] {
	const last = new Date(Date.UTC(year, month, 0)).getUTCDate();
	const days: number[] = [];
	for (let day = 1; day <= last; day += 1) {
		const weekday = new Date(Date.UTC(year, month - 1, day)).getUTCDay();
		if (weekday !== 0 && weekday !== 6) days.push(day);
	}
	return days;
}

function writtenDays(year: number, month: number, firstSchoolDay: number): number[] {
	return dayNumbersForSlots(year, month, firstSchoolDay, BUNDLED_SLOTS)
		.map((entry) => entry.day)
		.filter((day): day is number => day !== undefined);
}

// ── The form's own geometry ──────────────────────────────────────────────────

let fixture: TemplateFixture;

beforeEach(async () => {
	fixture = await loadTemplate();
});

describe('the bundled template', () => {
	it('labels exactly the 25 day columns the day grid assumes', async () => {
		const workbook = await fixture.open();
		const slots = weekdaySlots(workbook.getWorksheet(TEMPLATE_SHEETS[0]) as Worksheet);
		expect(slots.map((slot) => slot.column)).toEqual(LABELLED_COLUMNS);
		expect(slots.map((slot) => slot.weekdayIndex)).toEqual([
			0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4, 0, 1, 2, 3, 4
		]);
		expect(slots.map((slot) => slot.weekIndex)).toEqual([
			0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4
		]);
	});
	it('leaves the merged sub-columns out of the writable set', async () => {
		const workbook = await fixture.open();
		const writable = writableDayColumns(workbook.getWorksheet(TEMPLATE_SHEETS[0]) as Worksheet);
		expect(writable).toHaveLength(25);
		for (const skipped of [7, 13, 19, 23, 27, 34, 38])
			expect(writable).not.toContain(columnOf(skipped));
	});
});

function columnOf(column: number): string {
	let letter = '';
	let remaining = column;
	while (remaining > 0) {
		letter = String.fromCharCode(65 + ((remaining - 1) % 26)) + letter;
		remaining = Math.floor((remaining - 1) / 26);
	}
	return letter;
}

// ── Day numbers ──────────────────────────────────────────────────────────────

describe('the day grid', () => {
	it('leaves a Monday before classes start blank', () => {
		// 2026-09-01 is a Tuesday, so F (Monday, week 0) is blank and the week
		// starts on H.
		const byColumn = new Map(
			dayNumbersForSlots(2026, 9, 1, BUNDLED_SLOTS).map((e) => [e.column, e.day])
		);
		expect(byColumn.get(6)).toBeUndefined();
		expect(byColumn.get(8)).toBe(1);
		expect(byColumn.get(9)).toBe(2);
		expect(byColumn.get(10)).toBe(3);
		expect(byColumn.get(11)).toBe(4);
		expect(byColumn.get(12)).toBe(7);
		expect(byColumn.get(16)).toBe(10);
		expect(byColumn.get(17)).toBe(11);
	});

	it('puts every written day on its own weekday column', () => {
		for (const [month, year] of [
			[9, 2026],
			[2, 2027],
			[6, 2027],
			[8, 2027]
		]) {
			for (const { column, day } of dayNumbersForSlots(year, month, 1, BUNDLED_SLOTS)) {
				if (day === undefined) continue;
				const weekday = (new Date(Date.UTC(year, month - 1, day)).getUTCDay() + 6) % 7;
				const slot = BUNDLED_SLOTS.find((entry) => entry.column === column);
				expect(weekday, `${year}-${month}: day ${day} landed on column ${column}`).toBe(
					slot?.weekdayIndex
				);
			}
		}
	});

	it('writes every school day of the month once, and no weekend', () => {
		expect(writtenDays(2027, 6, 1)).toEqual(schoolDays(2027, 6));
	});

	it('blanks the earlier columns when classes start late', () => {
		// Classes started on Tuesday 15 September 2026.
		const byColumn = new Map(
			dayNumbersForSlots(2026, 9, 15, BUNDLED_SLOTS).map((e) => [e.column, e.day])
		);
		expect(byColumn.get(6)).toBeUndefined();
		expect(byColumn.get(12)).toBe(21);
		expect(byColumn.get(14)).toBe(22);
		expect(writtenDays(2026, 9, 15)[0]).toBe(15);
	});

	it('still dates a month whose first attendance day is a weekend', () => {
		// 2026-09-06 is a Sunday: not a school day, and not a Monday to count weeks
		// from. Without stepping back to the previous school day the whole month
		// would come out empty.
		const written = writtenDays(2026, 9, 6);
		expect(written).toHaveLength(18);
		expect(written[0]).toBe(7);
		expect(written[written.length - 1]).toBe(30);
	});
});

describe('the DepEd day grid holds every school day', () => {
	it('drops no school day of any month of a century', () => {
		let checked = 0;
		for (let year = 2000; year <= 2100; year += 1) {
			for (let month = 1; month <= 12; month += 1) {
				for (const firstSchoolDay of schoolDays(year, month)) {
					const dropped = daysWithoutASlot(year, month, firstSchoolDay, BUNDLED_SLOTS);
					expect(
						dropped,
						`${year}-${String(month).padStart(2, '0')} from day ${firstSchoolDay}`
					).toEqual([]);
					checked += 1;
				}
			}
		}
		expect(checked).toBeGreaterThan(10_000);
	});

	it('still has room for the widest possible month', () => {
		let widest = 0;
		for (let year = 2000; year <= 2100; year += 1) {
			for (let month = 1; month <= 12; month += 1) {
				widest = Math.max(widest, schoolDays(year, month).length);
			}
		}
		expect(widest).toBe(23);
		expect(BUNDLED_SLOTS).toHaveLength(25);
	});
});

describe('the day grid becomes date mappings', () => {
	it('records one row per school day, on the column the sheet was written to', () => {
		const mappings = monthDateMappings('template-1', 2026, 9, 1, BUNDLED_SLOTS);
		expect(mappings).toHaveLength(22);
		expect(mappings[0]).toMatchObject({
			date: '2026-09-01',
			columnLetter: 'H',
			templateId: 'template-1',
			sheetName: 'SEPTEMBER 2026'
		});
		expect(mappings[mappings.length - 1].date).toBe('2026-09-30');
		for (const mapping of mappings)
			expect(columnOf(mapping.columnIndex)).toBe(mapping.columnLetter);
	});
});

// ── Roster bands and expansion ──────────────────────────────────────────────

describe('the roster bands', () => {
	const learners: Sf2LearnerRow[] = [
		{ row: 8, name: 'CRUZ, JUAN', gender: 'M' },
		{ row: 28, name: 'REYES, MARIA', gender: 'M' },
		{ row: 30, name: 'SANTOS, LIZA', gender: 'F' },
		{ row: 48, name: 'TAN, ANA', gender: 'F' }
	];

	it('never spans a TOTAL row', () => {
		// Rows 29 and 49 hold SUM formulas. Reading them would count phantom marks and
		// writing them back would be refused by the formula guard.
		const bands = attendanceBands(learners);
		expect(bands).toEqual([
			{ firstRow: 8, lastRow: 28, firstColumn: 6, lastColumn: 38 },
			{ firstRow: 30, lastRow: 48, firstColumn: 6, lastColumn: 38 }
		]);
		expect(attendanceBands(learners.slice(0, 2))).toHaveLength(1);
	});

	it('spans the whole day grid', () => {
		const band: RowBand = { firstRow: 8, lastRow: 28, firstColumn: 6, lastColumn: 38 };
		expect(band.lastColumn - band.firstColumn + 1).toBe(33);
		expect(band.lastRow - band.firstRow + 1).toBe(21);
	});

	it('splits a roster about to be written the same way', () => {
		expect(
			attendanceBandsForWrites([
				{ studentId: 'a', rowIndex: 8, name: 'CRUZ, JUAN', itemNumber: 1, genderBlock: 'MALE' },
				{ studentId: 'b', rowIndex: 30, name: 'TAN, ANA', itemNumber: 1, genderBlock: 'FEMALE' }
			])
		).toEqual(
			attendanceBands([
				{ row: 8, name: 'CRUZ, JUAN', gender: 'M' },
				{ row: 30, name: 'TAN, ANA', gender: 'F' }
			])
		);
	});

	it('finds the female block, or answers with the fresh template row', () => {
		expect(sourceFemaleStartRow(learners)).toBe(30);
		expect(sourceFemaleStartRow(learners.slice(0, 2))).toBe(30);
	});
});

describe('growing the roster to fit the class', () => {
	it('grows a source roster whose female block had already moved', () => {
		// The legacy file was already expanded, so its female block starts at row 34.
		// A month file that was not grown to match would put row 34 on a different
		// learner, and every copied X would land on the wrong student.
		expect(rosterExpansionFor(24, 19, 34)).toEqual({ extraMale: 4, extraFemale: 0 });
		expect(headerRowShift(4, 0)).toBe(4);
	});

	it('grows a roster longer than the form', () => {
		expect(rosterExpansionFor(28, 25, 30)).toEqual({ extraMale: 7, extraFemale: 6 });
		expect(headerRowShift(7, 6)).toBe(13);
	});

	it('leaves a roster within capacity alone', () => {
		expect(rosterExpansionFor(21, 19, 30)).toEqual({ extraMale: 0, extraFemale: 0 });
		expect(rosterExpansionFor(4, 3, 30)).toEqual({ extraMale: 0, extraFemale: 0 });
	});
});

// ── Verification ────────────────────────────────────────────────────────────

describe('verifying a month', () => {
	it('passes only on an exact match', () => {
		expect(verifyMonthBuild(12, 12, 40, 40)).toEqual({ verified: true });
		// One missing mark fails the month.
		expect(verifyMonthBuild(12, 11, 40, 40).verified).toBe(false);
		// So does one extra: a month holding more marks than its source is just as
		// likely to be somebody else's marks.
		expect(verifyMonthBuild(12, 13, 40, 40).verified).toBe(false);
		expect(verifyMonthBuild(12, 12, 40, 39).verified).toBe(false);
	});

	it('lets a mismatch win over a verification in the same file', () => {
		expect(combineVerifications([{ verified: true }, verifyMonthBuild(12, 11, 40, 40)])).toEqual({
			verified: false,
			expectedX: 12,
			foundX: 11,
			expectedLearners: 40,
			foundLearners: 40
		});
		expect(combineVerifications([{ verified: true }, { verified: true }])).toEqual({
			verified: true
		});
	});
});

describe('resolving absences onto the grid', () => {
	const learners: MonthLearnerWrite[] = [
		{ studentId: 'a', rowIndex: 8, name: 'CRUZ, JUAN', itemNumber: 1, genderBlock: 'MALE' },
		{ studentId: 'b', rowIndex: 9, name: 'REYES, MARIA', itemNumber: 2, genderBlock: 'MALE' }
	];
	const dates = monthDateMappings('t1', 2026, 9, 1, BUNDLED_SLOTS);
	const writable = LABELLED_COLUMNS;

	it('drops an absence naming a learner or a day the sheet cannot hold', () => {
		const resolved = resolveMarks(
			{
				templateId: 't1',
				reportMonth: 'SEPTEMBER',
				reportYear: 2026,
				firstSchoolDay: 1,
				header: {} as MonthBuildRequest['header'],
				learners,
				sourceFemaleStartRow: 30,
				absences: [
					{ studentId: 'a', date: '2026-09-02' },
					{ studentId: 'a', date: '2026-09-02' },
					{ studentId: 'nobody', date: '2026-09-03' },
					{ studentId: 'b', date: '2026-09-31' }
				]
			},
			dates,
			writable
		);
		// The duplicate is collapsed, and both unmappable absences are counted.
		expect(resolved.marks).toEqual([{ rowIndex: 8, columnIndex: 9, value: 'X' }]);
		expect(resolved.unmappedStudents).toBe(1);
		expect(resolved.unmappedDates).toBe(1);
	});
});

// ── Building a month sheet against the real template ─────────────────────────

/**
 * A class of `maleCount` boys and `femaleCount` girls, laid out the way the sheet
 * will hold them once it has grown to the roster's shape - which is why the female
 * start row is a parameter and not always 30.
 */
function roster(maleCount: number, femaleCount: number, femaleStartRow = 30): MonthLearnerWrite[] {
	const learners: MonthLearnerWrite[] = [];
	for (let index = 0; index < maleCount; index += 1) {
		learners.push({
			studentId: `m${index}`,
			rowIndex: 8 + index,
			name: `BOY ${index}, JUAN`,
			itemNumber: index + 1,
			genderBlock: 'MALE'
		});
	}
	for (let index = 0; index < femaleCount; index += 1) {
		learners.push({
			studentId: `f${index}`,
			rowIndex: femaleStartRow + index,
			name: `GIRL ${index}, ANA`,
			itemNumber: index + 1,
			genderBlock: 'FEMALE'
		});
	}
	return learners;
}

const HEADER: MonthBuildRequest['header'] = {
	schoolId: '132839',
	schoolName: 'ESPIRITU ELEMENTARY SCHOOL',
	schoolYear: '2026-2027',
	reportMonth: 'SEPTEMBER',
	gradeLevel: 'GRADE 3',
	section: 'MATAPAT',
	adviserName: 'DELA CRUZ, JUAN',
	schoolHeadName: 'SANTOS, MARIA'
};

function requestFor(
	learners: MonthLearnerWrite[],
	absences: { studentId: string; date: string }[]
): MonthBuildRequest {
	const females = learners.filter((learner) => learner.genderBlock === 'FEMALE');
	return {
		templateId: 'template-1',
		reportMonth: 'SEPTEMBER',
		reportYear: 2026,
		firstSchoolDay: 1,
		header: HEADER,
		learners,
		absences,
		sourceFemaleStartRow: females.length > 0 ? females[0].rowIndex : 30
	};
}

describe('building a month worksheet from the bundled template', () => {
	it('replaces the template sample sheets with the one month being written', async () => {
		const report = await buildSchoolYearWorkbook(fixture.path, [
			{ request: requestFor(roster(12, 14), []), removeStaleSheets: true }
		]);

		expect(sheetNamesOf(report)).toEqual(['SEPTEMBER 2026']);
		expect(report.verification).toEqual({ verified: true });
		// Every worksheet the bundled template ships carries the form title, so all of
		// them are retired - including its 36 sample X marks.
		expect(report.removedSheets).toEqual(TEMPLATE_SHEETS);
		expect(report.keptHelperSheets).toEqual([]);

		const reopened = await fixture.open();
		expect(reopened.worksheets.map((sheet) => sheet.name)).toEqual(['SEPTEMBER 2026']);
	});

	it('keeps the other months when the caller did not ask for a retire', async () => {
		await buildSchoolYearWorkbook(fixture.path, [
			{ request: requestFor(roster(12, 14), []), removeStaleSheets: false }
		]);
		const reopened = await fixture.open();
		expect(reopened.worksheets.map((sheet) => sheet.name)).toEqual([
			...TEMPLATE_SHEETS,
			'SEPTEMBER 2026'
		]);
	});

	it('rebuilds a month onto the worksheet its own last build wrote', async () => {
		const request = requestFor(roster(12, 14), []);
		await buildSchoolYearWorkbook(fixture.path, [{ request, removeStaleSheets: true }]);
		// "Create October" on a workbook that already holds October: the form has to be
		// copied over 681 merges that are already there.
		const again = await buildSchoolYearWorkbook(fixture.path, [
			{ request, removeStaleSheets: false }
		]);

		expect(again.verification).toEqual({ verified: true });
		const sheet = (await fixture.open()).getWorksheet('SEPTEMBER 2026') as Worksheet;
		expect(sheet.model.merges.length).toBe(681);
	});

	it('writes the class roster, the total labels and the day numbers', async () => {
		const learners = roster(12, 14);
		const report = await buildSchoolYearWorkbook(fixture.path, [
			{ request: requestFor(learners, []), removeStaleSheets: true }
		]);

		const sheet = (await fixture.open()).getWorksheet('SEPTEMBER 2026') as Worksheet;
		expect(readLearnerRows(sheet)).toEqual(
			learners.map((learner) => ({
				row: learner.rowIndex,
				name: learner.name,
				gender: learner.genderBlock === 'MALE' ? 'M' : 'F'
			}))
		);
		expect(getCellText(sheet, TEMPLATE_TOTALS.maleTotalRow, 3)).toBe('MALE TOTAL');
		expect(getCellText(sheet, TEMPLATE_TOTALS.femaleTotalRow, 3)).toBe('FEMALE TOTAL');
		expect(getCellText(sheet, TEMPLATE_TOTALS.combinedTotalRow, 3)).toBe('COMBINED TOTAL');

		// September 2026 starts on a Tuesday, so column F is blank and H carries day 1.
		expect(getCellText(sheet, 6, 6)).toBe('');
		expect(getCellText(sheet, 6, 8)).toBe('1');
		expect(getCellText(sheet, 6, 17)).toBe('11');
		expect(report.months[0].dates).toHaveLength(22);
		expect(report.months[0].extraRosterRows).toBe(0);
	});

	it('hides the learner rows the roster does not claim, so only the class prints', async () => {
		await buildSchoolYearWorkbook(fixture.path, [
			{ request: requestFor(roster(2, 1), []), removeStaleSheets: true }
		]);
		const sheet = (await fixture.open()).getWorksheet('SEPTEMBER 2026') as Worksheet;

		expect(sheet.getRow(8).hidden).toBe(false);
		expect(sheet.getRow(9).hidden).toBe(false);
		expect(sheet.getRow(30).hidden).toBe(false);
		expect(sheet.getRow(10).hidden).toBe(true);
		expect(sheet.getRow(28).hidden).toBe(true);
		expect(sheet.getRow(31).hidden).toBe(true);
		// The TOTAL rows are not slots and must never be hidden.
		expect(sheet.getRow(29).hidden).toBe(false);
		expect(sheet.getRow(49).hidden).toBe(false);
	});

	it('writes the header block, and shifts the signature rows when the roster grows', async () => {
		// 24 boys and 22 girls whose female block starts at row 33, because the source
		// roster had already been expanded: three extra rows each, so the adviser and
		// school head signature blocks move down six.
		await buildSchoolYearWorkbook(fixture.path, [
			{ request: requestFor(roster(24, 22, 33), []), removeStaleSheets: true }
		]);
		const sheet = (await fixture.open()).getWorksheet('SEPTEMBER 2026') as Worksheet;

		expect(getCellText(sheet, 3, 6)).toBe(HEADER.schoolId);
		expect(getCellText(sheet, 4, 39)).toBe(HEADER.section);
		// 21 + 3 male slots puts MALE TOTAL on 32; 19 + 3 female slots, FEMALE TOTAL on 55.
		expect(getCellText(sheet, 32, 3)).toBe('MALE TOTAL');
		expect(getCellText(sheet, 55, 3)).toBe('FEMALE TOTAL');
		expect(getCellText(sheet, 56, 3)).toBe('COMBINED TOTAL');
		expect(getCellText(sheet, 76 + 6, 40)).toBe(HEADER.adviserName);
		expect(getCellText(sheet, 82 + 6, 26)).toBe(HEADER.adviserName);
		expect(getCellText(sheet, 82 + 6, 40)).toBe(HEADER.schoolHeadName);
	});

	it('keeps every merge when the roster grows', async () => {
		// Growing a merged worksheet with ExcelJS's `spliceRows` loses the merges: the
		// slaves are re-pointed without being registered, so the ranges never reach the
		// file, and the master's text lands in every cell of the rectangle. The roster
		// block below the MALE TOTAL row came out as an unmerged grid.
		await buildSchoolYearWorkbook(fixture.path, [
			{ request: requestFor(roster(24, 22, 33), []), removeStaleSheets: true }
		]);
		const sheet = (await fixture.open()).getWorksheet('SEPTEMBER 2026') as Worksheet;

		expect(sheet.model.merges).toHaveLength(681);
		// The guidelines block is a four-row merge whose text must live in one cell.
		const guidelines = (sheet.getCell('A64') as { master?: { address: string } }).master;
		expect(guidelines?.address).toBe('A64');
		expect(getCellText(sheet, 65, 1)).toBe(getCellText(sheet, 64, 1));
	});

	it('writes each absence once, in the cell its date and learner resolve to', async () => {
		const learners = roster(12, 14);
		const absences = [
			{ studentId: 'm0', date: '2026-09-02' },
			{ studentId: 'f1', date: '2026-09-16' }
		];
		const report = await buildSchoolYearWorkbook(fixture.path, [
			{ request: requestFor(learners, absences), removeStaleSheets: true }
		]);

		const sheet = (await fixture.open()).getWorksheet('SEPTEMBER 2026') as Worksheet;
		expect(report.months[0].writtenMarks).toBe(2);
		expect(report.verification).toEqual({ verified: true });
		// The column each date landed in is the build's own grid, not a guess.
		const columnOfDate = (date: string) =>
			report.months[0].dates.find((entry) => entry.date === date)?.columnIndex ?? 0;
		expect(getCellText(sheet, 8, columnOfDate('2026-09-02'))).toBe('X');
		expect(getCellText(sheet, 31, columnOfDate('2026-09-16'))).toBe('X');
		expect(getCellText(sheet, 9, columnOfDate('2026-09-02'))).toBe('');
	});

	it('writes the ABSENT, PRESENT and TOTAL formulas with their cached values', async () => {
		await buildSchoolYearWorkbook(fixture.path, [
			{
				request: requestFor(roster(12, 14), [{ studentId: 'm0', date: '2026-09-02' }]),
				removeStaleSheets: true
			}
		]);
		const sheet = (await fixture.open()).getWorksheet('SEPTEMBER 2026') as Worksheet;

		// `AW5` is the multiplier every PRESENT formula uses, so it has to hold the
		// real mapped day count rather than the template's stale 11.
		expect(sheet.getCell('AW5').value).toBe(22);
		expect(formulaOf(sheet.getCell('AM8'))).toBe('COUNTIF(F8:AL8,"X")');
		expect(formulaResult(sheet.getCell('AM8'))).toBe(1);
		expect(formulaResult(sheet.getCell('AO8'))).toBe(21);
		// 2 September is column I: one boy absent, so 12 - 1 present.
		expect(formulaOf(sheet.getCell('I29'))).toBe('12-COUNTIF(I8:I28,"X")');
		expect(formulaResult(sheet.getCell('I29'))).toBe(11);
		expect(formulaResult(sheet.getCell('I49'))).toBe(14);
		expect(formulaResult(sheet.getCell('I50'))).toBe(25);
		expect(formulaResult(sheet.getCell('AM29'))).toBe(1);
		expect(formulaResult(sheet.getCell('AO29'))).toBe(22 * 12 - 1);
	});

	it('refuses to save a month whose marks did not land, and leaves the file alone', async () => {
		const before = fixture.fileSystem.files.get(fixture.path);
		// An absence on a day the DepEd grid has no column for: 2026-09-06 is a Sunday.
		const report = await buildSchoolYearWorkbook(fixture.path, [
			{
				request: requestFor(roster(12, 14), [{ studentId: 'm0', date: '2026-09-06' }]),
				removeStaleSheets: false
			}
		]);

		expect(report.verification.verified).toBe(false);
		expect(report.months[0].unmappedAbsences).toEqual({ students: 0, dates: 1 });
		expect(fixture.fileSystem.files.get(fixture.path)).toBe(before);
		expect((await fixture.open()).worksheets.map((sheet) => sheet.name)).toEqual(TEMPLATE_SHEETS);
	});

	it('refuses a report month the workbook cannot hold', async () => {
		await expect(
			buildSchoolYearWorkbook(fixture.path, [
				{ request: { ...requestFor([], []), reportMonth: 'SEMESTER' }, removeStaleSheets: true }
			])
		).rejects.toThrow(/is not a month this workbook can hold/);
	});
});

describe('reading the legacy workbook back', () => {
	it('reads one month without touching the other eleven', async () => {
		const reads = await readLegacyMonths(fixture.path, [
			{ reportMonth: 'june', reportYear: 2025 },
			{ reportMonth: 'november', reportYear: 2026 }
		]);

		expect(reads[0].error).toBeUndefined();
		expect(reads[0].snapshot?.sheetName).toBe('JUNE 2025');
		// The 13 X marks the bundled `JUNE 2025` sheet holds on named learner rows:
		// five among the boys and eight among the girls. The Rust `read_legacy_month`
		// counted over `attendance_bands` - one band per gender block, never
		// `first..=last` over the sheet, because the MALE/FEMALE TOTAL rows between
		// them hold `SUM` formulas whose evaluated values are numbers. Counting the
		// whole learner span instead would read those two totals as phantom marks.
		expect(reads[0].snapshot?.xCount).toBe(13);
		expect(reads[0].snapshot?.learners).toHaveLength(
			TEMPLATE_ROSTER.maleCount + TEMPLATE_ROSTER.femaleCount
		);
		expect(reads[0].snapshot?.femaleStartRow).toBe(30);

		// A month with no sheet is its own failed outcome, not a failed read.
		expect(reads[1].snapshot).toBeUndefined();
		expect(reads[1].error).toMatch(/no NOVEMBER sheet to split from/);
	});

	it('turns a mark column back into the date the sheet printed there', async () => {
		const [june] = await readLegacyMonths(fixture.path, [
			{ reportMonth: 'JUNE', reportYear: 2025 }
		]);
		const snapshot = june.snapshot;
		if (!snapshot) throw new Error('the bundled template has a JUNE 2025 sheet');

		for (const [column, day] of snapshot.dayByColumn) {
			expect(dateInColumn(snapshot, column)).toBe(`2025-06-${String(day).padStart(2, '0')}`);
		}
		// A merged pair's unlabelled sub-column carries no day, so it cannot be dated.
		expect(dateInColumn(snapshot, 7)).toBeUndefined();
		// Neither can a column past the month's last day.
		expect(dateInColumn(snapshot, 39)).toBeUndefined();
	});
});
