/**
 * The golden round trip: the real converted DepEd template, written to and read
 * back, with nothing but an in-memory file system.
 *
 * This is the test that proves D7. If a formula mark went out without its cached
 * value the reopened cell would read `undefined` here rather than a number, which
 * is exactly how the app would then under-count the X marks it compares.
 */

import type { Cell, Workbook, Worksheet } from 'exceljs';
import { beforeEach, describe, expect, it } from 'vitest';
import { SF2_FIRST_LEARNER_ROW } from '../constants';
import {
	learnerAbsentPresentFormulaMarks,
	summaryFormulaMarks,
	totalFormulaMarks,
	type Sf2DayColumn
} from '../formula-marks';
import {
	clearTotalRows,
	writeFormulaMarks,
	writeMarks,
	writeMarksForce,
	writeMetadata
} from '../marks';
import { readLearnerRows } from '../roster';
import {
	activateSheet,
	formulaOf,
	formulaResult,
	getCellText,
	hasFormula,
	readDayGrid,
	writableDayColumns
} from '../workbook';
import type { Sf2CellMark, Sf2WorkbookMetadata } from '../types';
import {
	loadTemplate,
	TEMPLATE_DAY_COUNT,
	TEMPLATE_ROSTER,
	TEMPLATE_SHEETS,
	TEMPLATE_TOTALS,
	type TemplateFixture
} from './template-fixture';

const SHEET = TEMPLATE_SHEETS[0];

/**
 * The 21 school days JUNE 2025 maps, as `{ sheetName, column }`.
 *
 * Not every column in `F:AL` is addressable: the form merges consecutive day
 * columns into pairs so a weekend or non-school day occupies the same width as a
 * school one, and only the left column of each pair holds the day number.
 */
const JUNE_DAYS: Sf2DayColumn[] = [
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
	'AF'
].map((column) => ({ sheetName: SHEET, column }));

const METADATA: Sf2WorkbookMetadata = {
	schoolId: '132839',
	schoolName: 'ESPIRITU ELEMENTARY SCHOOL',
	schoolYear: '2025-2026',
	reportMonth: 'JUNE',
	gradeLevel: 'GRADE 3',
	section: 'MATAPAT',
	adviserName: 'DELA CRUZ, JUAN',
	schoolHeadName: 'SANTOS, MARIA',
	firstSchoolDay: 1
};

/**
 * The value Excel cached for a formula cell.
 *
 * Read from the model, not `cell.value`: ExcelJS's value getter copies through a
 * truthiness check, so a cached result of `0` would come back as `undefined` and
 * the test would pass on a cell the app would read as uncalculated.
 */
function cachedOf(cell: Cell): number | string | boolean | undefined {
	return formulaResult(cell);
}

/** The value a cell reads back as: a formula's cached result, or its literal. */
function storedValue(cell: Cell): unknown {
	return hasFormula(cell) ? formulaResult(cell) : cell.value;
}

function gridSource(workbook: Workbook) {
	return (sheetName: string) => readDayGrid(workbook.getWorksheet(sheetName) as Worksheet, 8, 50);
}

/** Mean of the numeric cells of a TOTAL row, worked out here rather than by the module under test. */
function meanOfRow(workbook: Workbook, row: number): number {
	const numbers =
		readDayGrid(workbook.getWorksheet(SHEET) as Worksheet, row, row).numbersByRow.get(row) ?? [];
	return numbers.reduce((total, value) => total + value, 0) / numbers.length;
}

let fixture: TemplateFixture;

beforeEach(async () => {
	fixture = await loadTemplate();
});

describe('opening the real template', () => {
	it('finds the six worksheets Excel wrote', async () => {
		const workbook = await fixture.open();
		expect(workbook.worksheets.map((sheet) => sheet.name)).toEqual(TEMPLATE_SHEETS);
	});

	it('preserves the merged metadata cells the form is built from', async () => {
		const workbook = await fixture.open();
		const merges = new Set(workbook.getWorksheet(SHEET)?.model.merges ?? []);
		// School id, school name, section, adviser signature, school head - all merged
		// regions whose top-left corner is the only writable cell.
		for (const merge of ['F3:I3', 'F4:R4', 'AM4:AU4', 'AN76:AU77', 'AN82:AT82']) {
			expect(merges).toContain(merge);
		}
	});
});

describe('the golden round trip', () => {
	it('survives a write, an atomic save and a reopen', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.getWorksheet(SHEET) as Worksheet;
		const mergesBefore = sheet.model.merges.length;

		// Two absences the app would write, and one cell it is allowed to clear.
		const marks: Sf2CellMark[] = [
			{ sheetName: SHEET, address: 'U8', value: 'X' },
			{ sheetName: SHEET, address: 'R32', value: 'X' },
			{ sheetName: SHEET, address: 'U9', value: '' }
		];
		writeMarks(workbook, marks);

		const reopened = await fixture.roundTrip(workbook);
		const after = reopened.getWorksheet(SHEET) as Worksheet;

		expect(getCellText(after, 8, 21)).toBe('X'); // U8
		expect(getCellText(after, 32, 18)).toBe('X'); // R32
		expect(getCellText(after, 9, 21)).toBe(''); // U9, cleared
		expect(after.model.merges).toHaveLength(mergesBefore);
		expect(reopened.worksheets.map((entry) => entry.name)).toEqual(TEMPLATE_SHEETS);
	});

	it('reads back the same roster it read in', async () => {
		const workbook = await fixture.open();
		const before = readLearnerRows(workbook.getWorksheet(SHEET) as Worksheet);
		const after = readLearnerRows(
			(await fixture.roundTrip(workbook)).getWorksheet(SHEET) as Worksheet
		);

		expect(after).toEqual(before);
		expect(before[0]).toMatchObject({ row: SF2_FIRST_LEARNER_ROW, gender: 'M' });
		expect(before.filter((learner) => learner.gender === 'M')).toHaveLength(
			TEMPLATE_ROSTER.maleCount
		);
		expect(before.filter((learner) => learner.gender === 'F')).toHaveLength(
			TEMPLATE_ROSTER.femaleCount
		);
	});
});

describe('formula marks carry their cached value', () => {
	it('writes the ABSENT/PRESENT block with a number in every cell', async () => {
		const workbook = await fixture.open();
		const learners = readLearnerRows(workbook.getWorksheet(SHEET) as Worksheet);

		const { formulaMarks, staticMarks } = learnerAbsentPresentFormulaMarks(
			[SHEET],
			learners.map((learner) => ({ row: learner.row })),
			TEMPLATE_ROSTER.maleCount,
			TEMPLATE_ROSTER.femaleCount,
			TEMPLATE_DAY_COUNT,
			TEMPLATE_TOTALS,
			gridSource(workbook)
		);
		writeFormulaMarks(workbook, formulaMarks);
		writeMarksForce(workbook, staticMarks);

		const sheet = (await fixture.roundTrip(workbook)).getWorksheet(SHEET) as Worksheet;

		for (const entry of formulaMarks) {
			expect(typeof cachedOf(sheet.getCell(entry.address)), entry.address).toBe('number');
			expect(cachedOf(sheet.getCell(entry.address))).toBe(entry.cachedValue);
		}
		// `AW5` is the multiplier every PRESENT formula uses.
		expect(storedValue(sheet.getCell('AW5'))).toBe(TEMPLATE_DAY_COUNT);
		// Row 8 is absent once in the template, so 11 school days minus one.
		expect(formulaOf(sheet.getCell('AM8'))).toBe('COUNTIF(F8:AL8,"X")');
		expect(cachedOf(sheet.getCell('AM8'))).toBe(1);
		expect(formulaOf(sheet.getCell('AO8'))).toBe('$AW$5-AM8');
		expect(cachedOf(sheet.getCell('AO8'))).toBe(TEMPLATE_DAY_COUNT - 1);
		// Five X marks among the boys and eight among the girls - the 13 the bundled
		// `JUNE 2025` sheet really holds on named learner rows, which is the count the
		// Rust port documented for this template. The block is regenerated from the grid
		// rather than read off the template, so the template's own cache agreeing here is
		// the cross-check that the recomputation is reading the same marks Excel does.
		// (`AW5` above is the part that really is stale: the form's 11 days against 22.)
		expect(cachedOf(sheet.getCell('AM29'))).toBe(5);
		expect(cachedOf(sheet.getCell('AO29'))).toBe(TEMPLATE_DAY_COUNT * 12 - 5);
		expect(cachedOf(sheet.getCell('AM49'))).toBe(8);
		expect(cachedOf(sheet.getCell('AO49'))).toBe(TEMPLATE_DAY_COUNT * 14 - 8);
		expect(cachedOf(sheet.getCell('AM50'))).toBe(13);
		expect(cachedOf(sheet.getCell('AO50'))).toBe(
			TEMPLATE_DAY_COUNT * 12 - 5 + (TEMPLATE_DAY_COUNT * 14 - 8)
		);
	});

	it('writes the TOTAL Per Day rows as present counts, cached', async () => {
		const workbook = await fixture.open();
		const marks = totalFormulaMarks(
			JUNE_DAYS,
			TEMPLATE_ROSTER.maleCount,
			TEMPLATE_ROSTER.femaleCount,
			TEMPLATE_TOTALS,
			gridSource(workbook)
		);
		clearTotalRows(workbook, SHEET, TEMPLATE_TOTALS);
		writeFormulaMarks(workbook, marks);

		const sheet = (await fixture.roundTrip(workbook)).getWorksheet(SHEET) as Worksheet;

		// Column R holds one X among the boys (row 18) and one among the girls (row 32).
		expect(formulaOf(sheet.getCell('R29'))).toBe('12-COUNTIF(R8:R28,"X")');
		expect(cachedOf(sheet.getCell('R29'))).toBe(11);
		expect(formulaOf(sheet.getCell('R49'))).toBe('14-COUNTIF(R30:R48,"X")');
		expect(cachedOf(sheet.getCell('R49'))).toBe(13);
		expect(formulaOf(sheet.getCell('R50'))).toBe('R29+R49');
		expect(cachedOf(sheet.getCell('R50'))).toBe(24);
		// Column T has one X among the boys and none among the girls.
		expect(cachedOf(sheet.getCell('T29'))).toBe(11);
		expect(cachedOf(sheet.getCell('T49'))).toBe(14);
		expect(cachedOf(sheet.getCell('T50'))).toBe(25);
	});

	it('writes the summary block with cached values on all three columns', async () => {
		const workbook = await fixture.open();
		// Worked out from the template's own numbers, not from the module under test.
		const expectedAverage = {
			AR: meanOfRow(workbook, TEMPLATE_TOTALS.maleTotalRow),
			AS: meanOfRow(workbook, TEMPLATE_TOTALS.femaleTotalRow),
			AT: meanOfRow(workbook, TEMPLATE_TOTALS.combinedTotalRow)
		};

		const { formulaMarks, staticMarks } = summaryFormulaMarks(
			[SHEET],
			TEMPLATE_ROSTER.maleCount,
			TEMPLATE_ROSTER.femaleCount,
			TEMPLATE_TOTALS,
			gridSource(workbook),
			{
				AR: { lateEnrolment: 1, droppedOut: 2, transferredOut: 1, transferredIn: 2 },
				AS: { lateEnrolment: 0, droppedOut: 1, transferredOut: 0, transferredIn: 1 },
				AT: { lateEnrolment: 1, droppedOut: 3, transferredOut: 1, transferredIn: 3 }
			}
		);
		writeFormulaMarks(workbook, formulaMarks);
		writeMarksForce(workbook, staticMarks);

		const sheet = (await fixture.roundTrip(workbook)).getWorksheet(SHEET) as Worksheet;

		expect(formulaOf(sheet.getCell('AR59'))).toBe('AR53+AR55-AR67-AR69+AR71');
		expect(cachedOf(sheet.getCell('AR59'))).toBe(12);
		expect(formulaOf(sheet.getCell('AS59'))).toBe('AS53+AS55-AS67-AS69+AS71');
		expect(cachedOf(sheet.getCell('AS59'))).toBe(14);
		expect(cachedOf(sheet.getCell('AT59'))).toBe(26);

		expect(formulaOf(sheet.getCell('AR61'))).toBe('IF(AR53>0,AR59/AR53*100,0)');
		expect(cachedOf(sheet.getCell('AR61'))).toBe(100);
		expect(cachedOf(sheet.getCell('AT61'))).toBe(100);

		expect(formulaOf(sheet.getCell('AR63'))).toBe('IFERROR(AVERAGE(F29:AL29),0)');
		expect(cachedOf(sheet.getCell('AR63'))).toBeCloseTo(expectedAverage.AR, 10);
		expect(cachedOf(sheet.getCell('AS63'))).toBeCloseTo(expectedAverage.AS, 10);
		expect(cachedOf(sheet.getCell('AT63'))).toBeCloseTo(expectedAverage.AT, 10);

		expect(formulaOf(sheet.getCell('AR65'))).toBe('IF(AR59>0,AR63/AR59*100,0)');
		expect(cachedOf(sheet.getCell('AR65'))).toBeCloseTo((expectedAverage.AR / 12) * 100, 10);

		expect(cachedOf(sheet.getCell('AR53'))).toBeUndefined();
		expect(storedValue(sheet.getCell('AR53'))).toBe(TEMPLATE_ROSTER.maleCount);
		expect(storedValue(sheet.getCell('AS53'))).toBe(TEMPLATE_ROSTER.femaleCount);
		expect(storedValue(sheet.getCell('AT53'))).toBe(26);
	});
});

describe('the merged day-column pairs', () => {
	it('are skipped, because a slave write lands on its master', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.getWorksheet(SHEET) as Worksheet;
		// `F8:G8` and `R29:S29` are merged regions in the bundled template.
		expect(writableDayColumns(sheet)).toContain('F');
		expect(writableDayColumns(sheet)).not.toContain('G');
		expect(writableDayColumns(sheet)).toContain('R');
		expect(writableDayColumns(sheet)).not.toContain('S');
		expect(writableDayColumns(sheet)).not.toContain('AL');
	});

	it('refuses two marks that would land on one merged cell', async () => {
		const workbook = await fixture.open();
		// `R29` and `S29` are one merged cell; two different answers for it is a bug.
		expect(() =>
			writeFormulaMarks(workbook, [
				{
					sheetName: SHEET,
					address: 'R29',
					value: '11',
					formula: '12-COUNTIF(R8:R28,"X")',
					cachedValue: 11
				},
				{
					sheetName: SHEET,
					address: 'S29',
					value: '10',
					formula: '12-COUNTIF(S8:S28,"X")',
					cachedValue: 10
				}
			])
		).toThrow(/Marks disagree about JUNE 2025!R29/);
	});
});

describe('writing the metadata block', () => {
	it('lands on the merged cells of every form sheet, including COMPLETE DAYS', async () => {
		const workbook = await fixture.open();
		expect(writeMetadata(workbook, METADATA)).toBe(TEMPLATE_SHEETS.length);

		const reopened = await fixture.roundTrip(workbook);
		for (const sheetName of TEMPLATE_SHEETS) {
			const sheet = reopened.getWorksheet(sheetName) as Worksheet;
			expect(getCellText(sheet, 3, 6), `${sheetName}!F3`).toBe(METADATA.schoolId);
			expect(getCellText(sheet, 3, 13), `${sheetName}!M3`).toBe(METADATA.schoolYear);
			expect(getCellText(sheet, 3, 27), `${sheetName}!AA3`).toBe(METADATA.reportMonth);
			expect(getCellText(sheet, 4, 6), `${sheetName}!F4`).toBe(METADATA.schoolName);
			expect(getCellText(sheet, 4, 27), `${sheetName}!AA4`).toBe(METADATA.gradeLevel);
			expect(getCellText(sheet, 4, 39), `${sheetName}!AM4`).toBe(METADATA.section);
			expect(getCellText(sheet, 76, 40), `${sheetName}!AN76`).toBe(METADATA.adviserName);
			expect(getCellText(sheet, 82, 40), `${sheetName}!AN82`).toBe(METADATA.schoolHeadName);
		}
	});

	it('writes a text format so a school id keeps its leading digits', async () => {
		const workbook = await fixture.open();
		writeMetadata(workbook, METADATA);
		const sheet = (await fixture.roundTrip(workbook)).getWorksheet(SHEET) as Worksheet;
		expect(sheet.getCell('F3').numFmt).toBe('@');
	});
});

describe('refusing to clobber a formula', () => {
	it('throws when a literal mark would replace a formula cell', async () => {
		const workbook = await fixture.open();
		// AM29 ships as `SUM(AM8:AN28)`.
		expect(() => writeMarks(workbook, [{ sheetName: SHEET, address: 'AM29', value: '5' }])).toThrow(
			/Refusing to overwrite formula cell JUNE 2025!AM29/
		);
	});

	it('allows it when the caller means to force the value', async () => {
		const workbook = await fixture.open();
		writeMarksForce(workbook, [{ sheetName: SHEET, address: 'AM29', value: '5' }]);
		const cell = (workbook.getWorksheet(SHEET) as Worksheet).getCell('AM29');
		expect(formulaOf(cell)).toBeUndefined();
		expect(cell.value).toBe(5);
	});
});

describe('activating a sheet', () => {
	it('points the workbook at the tab the teacher asked for, and keeps it', async () => {
		const workbook = await fixture.open();
		expect(activateSheet(workbook, 'OCTOBER 2025')).toBe(true);
		expect(workbook.views[0]?.activeTab).toBe(TEMPLATE_SHEETS.indexOf('OCTOBER 2025'));

		const reopened = await fixture.roundTrip(workbook);
		expect(reopened.views[0]?.activeTab).toBe(TEMPLATE_SHEETS.indexOf('OCTOBER 2025'));
	});

	it('reports a miss instead of throwing', async () => {
		const workbook = await fixture.open();
		expect(activateSheet(workbook, 'NOVEMBER 2025')).toBe(false);
	});
});

describe('the atomic write', () => {
	it('leaves no temp file behind', async () => {
		const workbook = await fixture.open();
		await fixture.save(workbook);
		expect([...fixture.fileSystem.files.keys()]).toEqual([fixture.path]);
	});

	it('does not truncate the file when the write fails', async () => {
		const workbook = await fixture.open();
		const good = fixture.fileSystem.files.get(fixture.path);
		expect(good?.byteLength).toBeGreaterThan(0);

		// A worksheet reference that does not exist must abort the whole save rather
		// than leave a half-written workbook in place of the teacher's.
		const { writeMarks: write } = await import('../marks');
		expect(() =>
			write(workbook, [{ sheetName: 'NOVEMBER 2025', address: 'U8', value: 'X' }])
		).toThrow(/no worksheet named/);
		expect(fixture.fileSystem.files.get(fixture.path)).toBe(good);
	});
});
