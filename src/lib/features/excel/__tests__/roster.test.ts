// @vitest-environment node
/**
 * Roster row arithmetic, and workbook analysis.
 *
 * The row numbers here are the risk: one row off and the TOTAL rows, the COUNTIF
 * ranges and the teacher's printed report all shift together, silently. The
 * bundled template is the ground truth, so most of these assertions are made
 * against it rather than against a hand-built sheet.
 */

import type { Worksheet } from 'exceljs';
import { beforeEach, describe, expect, it } from 'vitest';
import {
	analyzeWorkbook,
	analyzeWorkbookFile,
	isSf2FormSheet,
	looksLikeSf2,
	SAMPLE_CELL_LIMIT
} from '../analysis';
import {
	bundledTemplateTotalRows,
	rosterExpansionNeeded,
	SF2_FIRST_LEARNER_ROW,
	SF2_FRESH_COMBINED_TOTAL_ROW,
	SF2_FRESH_FEMALE_TOTAL_ROW,
	SF2_FRESH_FEMALE_START_ROW,
	SF2_FRESH_FEMALE_SLOTS,
	SF2_FRESH_MALE_SLOTS,
	SF2_FRESH_MALE_TOTAL_ROW
} from '../constants';
import {
	depedLearnerIdFromCells,
	expandRosterRows,
	hideEmptyLearnerRows,
	isLearnerName,
	learnerSlots,
	readLearnerRows
} from '../roster';
import {
	columnLetter,
	columnNumber,
	monthName,
	monthNumber,
	sf2MonthlySheets,
	yearFromSheetName
} from '../workbook';
import {
	loadTemplate,
	newWorkbook,
	TEMPLATE_SHEETS,
	type TemplateFixture
} from './template-fixture';

const SHEET = TEMPLATE_SHEETS[0];

let fixture: TemplateFixture;

beforeEach(async () => {
	fixture = await loadTemplate();
});

describe('the SF2 row arithmetic', () => {
	it('places the TOTAL rows of a fresh template at 29, 49 and 50', () => {
		expect(bundledTemplateTotalRows(0, 0)).toEqual({
			maleTotalRow: SF2_FRESH_MALE_TOTAL_ROW,
			femaleTotalRow: SF2_FRESH_FEMALE_TOTAL_ROW,
			combinedTotalRow: SF2_FRESH_COMBINED_TOTAL_ROW
		});
		expect(SF2_FRESH_MALE_SLOTS + SF2_FIRST_LEARNER_ROW).toBe(SF2_FRESH_MALE_TOTAL_ROW);
		expect(SF2_FRESH_FEMALE_START_ROW + SF2_FRESH_FEMALE_SLOTS).toBe(SF2_FRESH_FEMALE_TOTAL_ROW);
		expect(SF2_FRESH_COMBINED_TOTAL_ROW).toBe(SF2_FRESH_FEMALE_TOTAL_ROW + 1);
	});

	it('leaves the template alone when the roster fits its slots', () => {
		expect(rosterExpansionNeeded(12, 14)).toEqual({ extraMale: 0, extraFemale: 0 });
		expect(rosterExpansionNeeded(21, 19)).toEqual({ extraMale: 0, extraFemale: 0 });
	});

	it('grows each block independently, by exactly the shortfall', () => {
		expect(rosterExpansionNeeded(25, 19)).toEqual({ extraMale: 4, extraFemale: 0 });
		expect(rosterExpansionNeeded(25, 22)).toEqual({ extraMale: 4, extraFemale: 3 });
	});

	it('pushes only the FEMALE and Combined rows down when there are more boys', () => {
		expect(bundledTemplateTotalRows(25, 19)).toEqual({
			maleTotalRow: 33,
			femaleTotalRow: 53,
			combinedTotalRow: 54
		});
	});

	it('enumerates the learner slots without ever naming a TOTAL row', () => {
		const slots = learnerSlots(SF2_FRESH_MALE_TOTAL_ROW, SF2_FRESH_FEMALE_TOTAL_ROW);
		expect(slots).toHaveLength(SF2_FRESH_MALE_SLOTS + SF2_FRESH_FEMALE_SLOTS);
		expect(slots[0]).toEqual({ row: 8, gender: 'M' });
		expect(slots.at(-1)).toEqual({ row: 48, gender: 'F' });
		expect(slots.map((slot) => slot.row)).not.toContain(SF2_FRESH_MALE_TOTAL_ROW);
		expect(slots.map((slot) => slot.row)).not.toContain(SF2_FRESH_FEMALE_TOTAL_ROW);
	});
});

describe('sheet-name rules', () => {
	it('reads the month and year out of a name, in every spelling the app produces', () => {
		for (const [name, month, year] of [
			['JUNE 2025', 6, 2025],
			['SEPT. 2025', 9, 2025],
			['SEPTEMBER 2026', 9, 2026],
			['JUNE2025', 6, 2025]
		] as [string, number, number][]) {
			expect(monthNumber(name), name).toBe(month);
			expect(yearFromSheetName(name), name).toBe(year);
		}
	});

	it('rejects a helper sheet and a wrong-century year', () => {
		expect(monthNumber('COMPLETE DAYS')).toBe(0);
		expect(monthNumber('__SF2_HIDDEN_1')).toBe(0);
		expect(yearFromSheetName('JUNE 1899')).toBe(0);
		expect(yearFromSheetName('JUNE 20255')).toBe(0);
	});

	it('names a month the way the workbook spells it', () => {
		expect(monthName(6)).toBe('JUNE');
		expect(monthName(9)).toBe('SEPTEMBER');
		expect(monthName(13)).toBe('');
	});

	it('round-trips a column number through its letters', () => {
		expect(columnLetter(1)).toBe('A');
		expect(columnLetter(26)).toBe('Z');
		expect(columnLetter(27)).toBe('AA');
		expect(columnLetter(38)).toBe('AL');
		expect(columnNumber('AL')).toBe(38);
		expect(columnNumber('al')).toBe(38);
	});
});

describe('recognising a learner name', () => {
	it('accepts the form the app writes', () => {
		expect(isLearnerName('CUARES,JAIRO, ESPIRITU')).toBe(true);
		expect(isLearnerName('  MONTILLA,MAELD, JR., MANLIMOS ')).toBe(true);
	});

	it("rejects the form's own labels", () => {
		expect(isLearnerName('No.')).toBe(false);
		expect(isLearnerName('NAME (Last Name, First Name, Middle Name)')).toBe(false);
		expect(isLearnerName('<=== MALE | TOTAL Per Day ===>')).toBe(false);
		expect(isLearnerName('Combined TOTAL Per Day')).toBe(false);
		expect(isLearnerName('   ')).toBe(false);
	});

	it('accepts the merged title prose, which is why the row scan has to be bounded', () => {
		// Row 2 is merged `A2:AU2`, so its slave cells answer with the top-left
		// text - full of commas and letters. Only the bounded scan in
		// `readLearnerRows` keeps that out of the roster, not this predicate.
		expect(isLearnerName('(This replaces Form 1, Form 2 & STS Form 4 - Absenteeism)')).toBe(true);
	});
});

describe('the DepEd learner ID', () => {
	it('is taken when the sheet really carries one', () => {
		expect(depedLearnerIdFromCells('1234567890', '1')).toBe('1234567890');
	});

	it('is refused when the cell only echoes the item number', () => {
		// The bundled template merges `A8:B8`, so column 2 reads back the item number.
		expect(depedLearnerIdFromCells('1', '1')).toBeUndefined();
		expect(depedLearnerIdFromCells('  ', '1')).toBeUndefined();
	});
});

describe('reading the roster off the real template', () => {
	it('reads 12 boys in rows 8-19 and 14 girls in rows 30-43', async () => {
		const workbook = await fixture.open();
		const learners = readLearnerRows(workbook.getWorksheet(SHEET) as Worksheet);
		const boys = learners.filter((learner) => learner.gender === 'M');
		const girls = learners.filter((learner) => learner.gender === 'F');

		expect(boys.map((learner) => learner.row)).toEqual([
			8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
		]);
		expect(girls[0]?.row).toBe(SF2_FRESH_FEMALE_START_ROW);
		expect(girls).toHaveLength(14);
		expect(girls.at(-1)?.row).toBe(43);
	});

	it('never reports a row at or below the FEMALE TOTAL divider', async () => {
		const workbook = await fixture.open();
		const learners = readLearnerRows(workbook.getWorksheet(SHEET) as Worksheet);
		// The footnote block below Combined TOTAL is merged prose full of commas.
		expect(learners.filter((learner) => learner.row >= 50)).toHaveLength(0);
	});

	it('reads no DepEd IDs, because the template merges the ID column into the item number', async () => {
		const workbook = await fixture.open();
		const learners = readLearnerRows(workbook.getWorksheet(SHEET) as Worksheet);
		expect(learners.some((learner) => learner.learnerId !== undefined)).toBe(false);
	});

	it('only treats the five month tabs as writable sheets', async () => {
		const workbook = await fixture.open();
		// `COMPLETE DAYS` is a full copy of the form but is not a month.
		expect(sf2MonthlySheets(workbook).map((sheet) => sheet.name)).toEqual(
			TEMPLATE_SHEETS.slice(0, 5)
		);
	});
});

describe('expanding the roster area', () => {
	it('does nothing when there is nothing to expand', async () => {
		const workbook = await fixture.open();
		expect(expandRosterRows(workbook, 0, 0)).toBe(0);
	});

	it('pushes the FEMALE and Combined rows down by the number of extra boys', async () => {
		const workbook = await fixture.open();
		expect(expandRosterRows(workbook, 2, 0)).toBe(5);

		const sheet = (await fixture.roundTrip(workbook)).getWorksheet(SHEET) as Worksheet;
		// `<=== FEMALE | TOTAL Per Day ===>` was row 49, now row 51.
		expect(sheet.getCell('C51').text).toContain('FEMALE');
		expect(sheet.getCell('C52').text).toContain('Combined TOTAL');
		// The row that used to hold the female total now holds a blank new slot.
		expect(readLearnerRows(sheet).filter((learner) => learner.gender === 'F')).toHaveLength(14);
	});

	it('honours the actual TOTAL row positions of an already-expanded workbook', async () => {
		const workbook = await fixture.open();
		// Pass the current positions, not the fresh-template defaults.
		expandRosterRows(workbook, 3, 0, SF2_FRESH_MALE_TOTAL_ROW, SF2_FRESH_FEMALE_TOTAL_ROW);
		const sheet = workbook.getWorksheet(SHEET) as Worksheet;
		expect(sheet.getCell('C52').text).toContain('FEMALE');
	});
});

describe('hiding empty learner rows', () => {
	it('hides every slot the roster does not claim', async () => {
		const workbook = await fixture.open();
		const learners = readLearnerRows(workbook.getWorksheet(SHEET) as Worksheet);
		const occupied = new Set(learners.map((learner) => learner.row));

		hideEmptyLearnerRows(workbook, SF2_FRESH_MALE_TOTAL_ROW, SF2_FRESH_FEMALE_TOTAL_ROW, occupied);

		const sheet = workbook.getWorksheet(SHEET) as Worksheet;
		for (const learner of learners)
			expect(sheet.getRow(learner.row).hidden, `row ${learner.row}`).toBe(false);
		for (let row = SF2_FIRST_LEARNER_ROW; row < SF2_FRESH_MALE_TOTAL_ROW; row += 1) {
			if (!occupied.has(row)) expect(sheet.getRow(row).hidden, `row ${row}`).toBe(true);
		}
		// The TOTAL rows are not slots and must never be hidden.
		expect(sheet.getRow(SF2_FRESH_MALE_TOTAL_ROW).hidden).toBe(false);
		expect(sheet.getRow(SF2_FRESH_FEMALE_TOTAL_ROW).hidden).toBe(false);
	});

	it('keeps the hide flags through a save and reopen', async () => {
		const workbook = await fixture.open();
		hideEmptyLearnerRows(
			workbook,
			SF2_FRESH_MALE_TOTAL_ROW,
			SF2_FRESH_FEMALE_TOTAL_ROW,
			new Set([8, 30])
		);
		const sheet = (await fixture.roundTrip(workbook)).getWorksheet(SHEET) as Worksheet;
		expect(sheet.getRow(8).hidden).toBe(false);
		expect(sheet.getRow(30).hidden).toBe(false);
		expect(sheet.getRow(9).hidden).toBe(true);
		expect(sheet.getRow(31).hidden).toBe(true);
	});
});

describe('analysing a workbook', () => {
	it('inventories every sheet with its merges and a formula sample', async () => {
		const workbook = await fixture.open();
		const analysis = analyzeWorkbook(workbook);

		expect(analysis.exists).toBe(true);
		expect(analysis.sheets.map((sheet) => sheet.name)).toEqual(TEMPLATE_SHEETS);
		const june = analysis.sheets[0];
		expect(june?.index).toBe(1);
		expect(june?.rowCount).toBe(83);
		expect(june?.columnCount).toBe(51);
		expect(june?.mergedRanges).toContain('R29:S29');
		expect(june?.hasProtection).toBe(false);
		expect(june?.sampleCells.length).toBeGreaterThan(0);
		// Formula cells are sampled first: a wrong count is a wrong range.
		expect(june?.sampleCells).toHaveLength(SAMPLE_CELL_LIMIT);
		expect(june?.sampleCells.every((sample) => sample.formula !== undefined)).toBe(true);
		expect(june?.sampleCells.map((sample) => sample.address)).toContain('AM8');
		expect(june?.sampleCells[0]).toMatchObject({ address: 'AM8', row: 8, column: 39 });
	});

	it('recognises the template as SF2, COMPLETE DAYS and all', async () => {
		const workbook = await fixture.open();
		expect(looksLikeSf2(workbook)).toBe(true);
		for (const sheetName of TEMPLATE_SHEETS) {
			expect(isSf2FormSheet(workbook.getWorksheet(sheetName) as Worksheet), sheetName).toBe(true);
		}
	});

	it('does not mistake a plain workbook for SF2', () => {
		const workbook = newWorkbook('Class List');
		const sheet = workbook.getWorksheet('Class List') as Worksheet;
		sheet.getCell('A1').value = 'Grade 3 attendance';
		expect(looksLikeSf2(workbook)).toBe(false);
		expect(isSf2FormSheet(workbook.getWorksheet('Class List') as Worksheet)).toBe(false);
	});

	it('reports a workbook that is not there rather than throwing', async () => {
		const analysis = await analyzeWorkbookFile('/Documents/EES-AMS/workbooks/nope.xlsx');
		expect(analysis).toEqual({ exists: false, sheets: [], looksLikeSf2: false });
	});
});
