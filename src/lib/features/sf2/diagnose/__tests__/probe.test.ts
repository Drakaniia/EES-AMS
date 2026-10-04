import { beforeEach, describe, expect, it } from 'vitest';
import { markCount, nameAt, probeWorkbook, yearFromSheetName, type RawWorkbook } from '../probe';
import {
	clearDayRow,
	dayColumns,
	loadTemplate,
	printDayRow,
	renameSheet,
	useTestWorkbookDir,
	writeDayFormula,
	writeX,
	type TemplateFixture
} from './install-fixture';

/**
 * The read-only pass over a workbook, against the real converted DepEd template.
 *
 * `src-tauri/src/sf2/diagnose/workbook_probe.rs` read a band through a `TEXTJOIN` it
 * evaluated itself and validated the token count of every answer. Neither survives the
 * port - ExcelJS hands over a grid - so what is tested here is the thing that replaces
 * them: the three reads that decide the answer, and the two traps that would silently
 * turn a real mark into a phantom or a real day into two.
 */

let fixture: TemplateFixture;

beforeEach(async () => {
	useTestWorkbookDir();
	fixture = await loadTemplate();
});

/** Probe the referenced workbook, exactly as a full run would. */
async function probeTemplate(): Promise<RawWorkbook> {
	return probeWorkbook(fixture.path);
}

describe('yearFromSheetName', () => {
	it('reads the year out of a sheet name', () => {
		expect(yearFromSheetName('SEPT. 2025')).toBe(2025);
		expect(yearFromSheetName('JUNE 2026')).toBe(2026);
		expect(yearFromSheetName('OCTOBER2027')).toBe(2027);
	});

	it('carries no year for a name that names none', () => {
		expect(yearFromSheetName('__SF2_HIDDEN_1')).toBeUndefined();
		expect(yearFromSheetName('COMPLETE DAYS')).toBeUndefined();
	});
});

describe('the day grid', () => {
	it('reads the day numbers off the template, one per merged pair', async () => {
		const workbook = await probeTemplate();
		const june = workbook.sheets[0];

		// The template's June 2025 grid runs 2, 3, 4, 5, 6, 9, 10, ... Each merge pair is
		// F:G, so `cellText` answers for G with F's number. Reading both halves would
		// report 33 day columns for a 25-slot grid and every day would appear twice.
		expect(june?.dayNumbers.map((entry) => entry.day).slice(0, 6)).toEqual([2, 3, 4, 5, 6, 9]);
		expect(new Set(june?.dayNumbers.map((entry) => entry.day)).size).toBe(june?.dayNumbers.length);
	});

	it('reports only columns that can address a cell', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.worksheets[0]!;
		const printed = printDayRow(sheet, [1, 2, 3]);
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();
		const columns = probed.sheets[0]?.dayNumbers.map((entry) => entry.column) ?? [];

		// Each reported column is a merge master, which is the only kind that can hold an
		// addressable cell. Reading the slave would report the same day twice and the
		// second address would land on the first.
		expect(columns).toEqual(printed.slice(0, 3));
		expect(new Set(columns).size).toBe(columns.length);
	});

	it('reports an empty grid when the day row is blank, rather than guessing', async () => {
		const workbook = await fixture.open();
		clearDayRow(workbook.worksheets[0]!);
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();

		expect(probed.sheets[0]?.dayNumbers).toEqual([]);
	});

	it('skips a day number no month could ever hold', async () => {
		const workbook = await fixture.open();
		printDayRow(workbook.worksheets[0]!, [1, 2, 32, 99]);
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();

		// The probe only knows the form's 1..31 sanity bound, exactly as the Rust
		// `day_numbers` did and as `sf2WeekdaySlots` does. Whether a *day* exists in the
		// month is a question for `buildDateMappings`, which knows the year and month -
		// tested in `rules.test.ts`. Deciding it here would need a year the probe
		// deliberately does not trust.
		expect(probed.sheets[0]?.dayNumbers.map((entry) => entry.day)).toEqual([1, 2]);
	});

	it('reads a day whose number is a formula, not just a literal', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.worksheets[0]!;
		const columns = printDayRow(sheet, [1, 2, 3]);
		writeDayFormula(sheet, columns[2]!, 'IF(TRUE,3,4)', 3);
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();

		expect(probed.sheets[0]?.dayNumbers.map((entry) => entry.day)).toEqual([1, 2, 3]);
	});

	it('drops a day whose cached result is zero, and keeps the rest of the month', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.worksheets[0]!;
		const columns = printDayRow(sheet, [1, 2, 3]);
		writeDayFormula(sheet, columns[2]!, 'SUM(0)', 0);
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();

		// A zero is not a date. It is also not a reason to refuse the whole month: the
		// other two days keep the grid, which is what keeps a day the teacher cannot
		// record an absence on from being invented *and* a readable month from reading
		// as unreadable.
		expect(probed.sheets[0]?.dayNumbers.map((entry) => entry.day)).toEqual([1, 2]);
	});

	it('reads a grid whose merge masters hold nothing, instead of falling over', async () => {
		const workbook = await fixture.open();
		clearDayRow(workbook.worksheets[0]!);
		await fixture.roundTrip(workbook);

		// `cell.text` throws `TypeError: Cannot read properties of null` on a merge cell
		// with no value, and the grid is 33 x 80 of them. `cellText` is what stops that.
		await expect(probeTemplate()).resolves.toBeDefined();
	});
});

describe('the X marks', () => {
	it('finds the sample mark the template ships with', async () => {
		const probed = await probeTemplate();
		const june = probed.sheets[0];

		// The template carries one sample `X`, in row 8 (the first male learner).
		expect(markCount(june!)).toBeGreaterThan(0);
		expect(june?.marks.every((mark) => mark.rowIndex >= 1 && mark.rowIndex <= 80)).toBe(true);
	});

	it('counts a written mark once, not once per half of its merged pair', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.worksheets[0]!;
		const before = markCount((await probeTemplate()).sheets[0]!);
		const column = dayColumns(sheet)[3]!;
		writeX(sheet, 12, column);
		await fixture.roundTrip(workbook);

		const after = (await probeTemplate()).sheets[0];

		expect(markCount(after!)).toBe(before + 1);
	});

	it('reads a lowercase x as the same mark', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.worksheets[0]!;
		const before = markCount((await probeTemplate()).sheets[0]!);
		sheet.getRow(12).getCell(dayColumns(sheet)[4]!).value = 'x';
		await fixture.roundTrip(workbook);

		expect(markCount((await probeTemplate()).sheets[0]!)).toBe(before + 1);
	});

	it('does not read the subtotal rows as marks', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.worksheets[0]!;
		const column = dayColumns(sheet)[0]!;
		// Rows 29/49/50 carry the form's own TOTAL formulas, which resolve to numbers.
		// A number is not an `X`.
		sheet.getRow(29).getCell(column).value = { formula: 'SUM(F8:F28)', result: 3 };
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();

		expect(probed.sheets[0]?.marks.some((mark) => mark.rowIndex === 29)).toBe(false);
	});

	it('stops at row 80, so the adviser block cannot contribute marks', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.worksheets[0]!;
		writeX(sheet, 81, dayColumns(sheet)[0]!);
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();

		expect(probed.sheets[0]?.marks.some((mark) => mark.rowIndex === 81)).toBe(false);
	});
});

describe("the sheet's own roster", () => {
	it("reads the template's learner rows", async () => {
		const probed = await probeTemplate();
		const june = probed.sheets[0];

		expect(nameAt(june!, 8)).toBe('CUARES,JAIRO, ESPIRITU');
		// 12 boys (rows 8-19) and 14 girls (rows 30-43) are the real roster.
		const real = june?.rosterNames.filter((row) => row.rowIndex >= 8 && row.rowIndex <= 48) ?? [];
		expect(real).toHaveLength(26);
	});

	it('rejects the form own labels in the learner rows', async () => {
		const probed = await probeTemplate();
		const june = probed.sheets[0];

		// `<=== MALE | TOTAL Per Day ===>` contains a comma and letters, so only the
		// form-label rule keeps it out of the roster, and that rule is shared with the
		// rest of the app so the two cannot disagree about who a learner is.
		expect(nameAt(june!, 29)).toBeUndefined();
		expect(nameAt(june!, 49)).toBeUndefined();
		expect(nameAt(june!, 50)).toBeUndefined();
		expect(nameAt(june!, 5)).toBeUndefined();
	});

	it('carries the five form paragraphs the shared learner rule lets through', async () => {
		const probed = await probeTemplate();
		const june = probed.sheets[0];
		const real = new Set(
			Array.from({ length: 12 }, (_, index) => 8 + index).concat(
				Array.from({ length: 14 }, (_, index) => 30 + index)
			)
		);
		const phantom = june?.rosterNames.filter((row) => !real.has(row.rowIndex)) ?? [];

		// The form's own subtitle (row 2) and its guidelines paragraph (rows 64-67) both
		// hold a comma and letters, so `isLearnerName` - the rule the whole app shares -
		// calls them learners. This is inherited, not introduced: the Rust probe read the
		// same column with the same rule. Harmless, because a phantom row never becomes a
		// comparison: the referenced file's roster comes from the database, and a name
		// match against `students` cannot hit a sentence.
		expect(phantom.map((row) => row.rowIndex)).toEqual([2, 64, 65, 66, 67]);
	});

	it('carries no student id, because only the database knows who a row is', async () => {
		const probed = await probeTemplate();

		expect(probed.sheets[0]?.rosterNames.every((row) => row.studentId === '')).toBe(true);
	});
});

describe('which worksheets are read', () => {
	it('reads every sheet the template ships, all six of them', async () => {
		const probed = await probeTemplate();

		expect(probed.sheets).toHaveLength(6);
		expect(probed.sheets.map((sheet) => sheet.sheetName)).toContain('COMPLETE DAYS');
	});

	it('reads a hidden sheet, which is eleven of twelve months on a real install', async () => {
		const workbook = await fixture.open();
		const sheet = workbook.worksheets[0]!;
		renameSheet(sheet, '__SF2_HIDDEN_1');
		sheet.state = 'hidden';
		printDayRow(sheet, [1, 2, 3]);
		writeX(sheet, 8, dayColumns(sheet)[0]!);
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();
		const hidden = probed.sheets.find((candidate) => candidate.sheetName === '__SF2_HIDDEN_1');

		expect(hidden).toBeDefined();
		expect(hidden?.visible).toBe(false);
		expect(hidden?.dayNumbers).toHaveLength(3);
		expect(markCount(hidden!)).toBeGreaterThan(0);
	});

	it('carries no month for a sheet whose name says none', async () => {
		const workbook = await fixture.open();
		renameSheet(workbook.worksheets[0]!, '__SF2_HIDDEN_1');
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();

		expect(probed.sheets[0]?.monthFromName).toBeUndefined();
	});

	it('resolves the abbreviated month names the template actually uses', async () => {
		const probed = await probeTemplate();

		expect(probed.sheets.map((sheet) => sheet.monthFromName)).toEqual([6, 7, 8, 9, 10, undefined]);
	});

	it('reports the report-month header cell verbatim without trusting it', async () => {
		const workbook = await fixture.open();
		// Every sheet in a file the pre-split cycle has been through carries the same
		// header, which is exactly why it is reported and not believed.
		for (const sheet of workbook.worksheets) {
			sheet.getRow(3).getCell(27).value = 'SEPTEMBER';
		}
		await fixture.roundTrip(workbook);

		const probed = await probeTemplate();

		expect(probed.sheets.every((sheet) => sheet.headerMonthLabel === 'SEPTEMBER')).toBe(true);
		// Believing it would have resolved four sample-data sheets to September.
		expect(probed.sheets.filter((sheet) => sheet.monthFromName === 9)).toHaveLength(1);
	});
});

describe('read-only', () => {
	it('leaves the file bytes exactly as they were', async () => {
		const before = await fixture.fileSystem.readFile(fixture.path);

		await probeWorkbook(fixture.path);

		const after = await fixture.fileSystem.readFile(fixture.path);
		expect(Array.from(after)).toEqual(Array.from(before));
	});

	it('rejects a file that is not a workbook, so the caller can report it', async () => {
		await fixture.fileSystem.writeFileAtomic('/Documents/EES-AMS/workbooks/notes.txt', 'hello');

		await expect(probeWorkbook('/Documents/EES-AMS/workbooks/notes.txt')).rejects.toThrow();
	});

	it('rejects a file that is not there', async () => {
		await expect(probeWorkbook('/Documents/EES-AMS/workbooks/absent.xlsx')).rejects.toThrow();
	});

	it('leaves a sheet untouched on disk, merges included', async () => {
		const before = await fixture.fileSystem.readFile(fixture.path);
		const workbook = await fixture.open();
		expect(workbook.worksheets[0]?.model.merges.length).toBeGreaterThan(0);

		await probeWorkbook(fixture.path);

		expect(Array.from(await fixture.fileSystem.readFile(fixture.path))).toEqual(Array.from(before));
	});
});
