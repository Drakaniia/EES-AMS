/**
 * The workbook half of a sync: the marks it writes, the formulas it regenerates, the
 * cells the differential clear withdraws, and the progress it reports while doing it.
 *
 * Driven against the real converted DepEd template on an in-memory file system, so
 * the merged day-column pairs, the merged metadata cells and the template's own stale
 * `X` marks are all present - a fixture that invented a simpler workbook would pass
 * exactly the writes that lose a term of marks.
 */
import { afterEach, describe, expect, it } from 'vitest';
import type { Worksheet } from 'exceljs';
import { date, mapping, SHEET } from './attendance-fixture';
import { useFileSystem } from '$lib/platform/fs';
import { loadTemplate } from '$lib/features/excel/__tests__/template-fixture';
import { cellText, formulaResult, getCellText } from '$lib/features/excel/workbook';
import { SF2_ABSENT_MARK } from '$lib/features/sf2/logic';
import { workbookXCells } from '../attendance/attendance-marks';
import type { Sf2GridCell } from '../attendance/attendance-marks';
import {
	reportWriteProgress,
	rosterShape,
	totalsOn,
	writeAttendanceToWorkbook,
	writePhases,
	WRITE_CHUNK_SIZE
} from '../attendance/attendance-write';
import type { Sf2ProgressUpdate } from '../progress';

afterEach(() => {
	useFileSystem(null);
});

describe('rosterShape / totalsOn', () => {
	it('counts the gender blocks and sorts the rows', () => {
		expect(rosterShape([mapping('s1', 30), mapping('s2', 8), mapping('s3', 9)])).toEqual({
			rows: [{ row: 8 }, { row: 9 }, { row: 30 }],
			maleCount: 2,
			femaleCount: 1
		});
	});

	it('reads the TOTAL rows off the sheet rather than assuming the fresh layout', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet(SHEET) as Worksheet;
		expect(totalsOn(sheet)).toEqual({
			maleTotalRow: 29,
			femaleTotalRow: 49,
			combinedTotalRow: 50
		});
	});
});

describe('writeAttendanceToWorkbook', () => {
	it('writes the absences the database proves', async () => {
		const fixture = await loadTemplate();
		const written = await writeAttendanceToWorkbook({
			target: {
				sourcePath: fixture.path,
				sheetName: SHEET,
				roster: [mapping('s1', 8)],
				dates: [date('01', 'H', 8)]
			},
			absentIdsFor: () => new Set(['s1'])
		});
		expect(written).toBe(1);

		const sheet = (await fixture.open()).getWorksheet(SHEET) as Worksheet;
		expect(getCellText(sheet, 8, 8)).toBe(SF2_ABSENT_MARK);
	});

	it('withdraws an X the database no longer holds, and only that one', async () => {
		const fixture = await loadTemplate();
		const target = {
			sourcePath: fixture.path,
			sheetName: SHEET,
			roster: [mapping('s1', 8)],
			dates: [date('01', 'H', 8)]
		};

		await writeAttendanceToWorkbook({ target, absentIdsFor: () => new Set(['s1']) });
		await writeAttendanceToWorkbook({ target, absentIdsFor: () => new Set<string>() });

		const sheet = (await fixture.open()).getWorksheet(SHEET) as Worksheet;
		expect(cellText(sheet.getCell('H8'))).toBe('');
		// A template mark on a row this roster does not map is out of scope, so the
		// clear never reaches it.
		expect(cellText(sheet.getCell('R32'))).toBe(SF2_ABSENT_MARK);
	});

	it('leaves every TOTAL and ABSENT/PRESENT cell carrying a number', async () => {
		const fixture = await loadTemplate();
		await writeAttendanceToWorkbook({
			target: {
				sourcePath: fixture.path,
				sheetName: SHEET,
				roster: [mapping('s1', 8), mapping('s2', 30)],
				dates: [date('01', 'H', 8), date('02', 'I', 9)]
			},
			absentIdsFor: () => new Set(['s1'])
		});

		const sheet = (await fixture.open()).getWorksheet(SHEET) as Worksheet;
		// Only the derived cells carry formulas; `H8` is the literal X the app wrote.
		for (const address of ['H29', 'H49', 'H50', 'AM8', 'AO8', 'AM29', 'AO50']) {
			expect(typeof formulaResult(sheet.getCell(address)), address).toBe('number');
		}
		// `AW5` is the multiplier every PRESENT formula uses, and the template's own
		// value is stale - so this is the assertion that the day count was corrected.
		expect(sheet.getCell('AW5').value).toBe(2);
		// The absence the app wrote is counted...
		expect(formulaResult(sheet.getCell('AM8'))).toBeGreaterThanOrEqual(1);
		// ...and PRESENT stays consistent with it: two mapped days × one mapped male
		// learner, less the absences row 8 holds (the template ships some of its own).
		expect(formulaResult(sheet.getCell('AO8'))).toBe(
			2 * 1 - Number(formulaResult(sheet.getCell('AM8')))
		);
	});

	it('makes the month the workbook reopens on', async () => {
		const fixture = await loadTemplate();
		await writeAttendanceToWorkbook({
			target: {
				sourcePath: fixture.path,
				sheetName: 'JULY 2025',
				roster: [mapping('s1', 8)],
				dates: [date('01', 'H', 8)]
			},
			absentIdsFor: () => new Set<string>()
		});

		const reopened = await fixture.open();
		expect(reopened.views[0]?.activeTab).toBe(
			reopened.worksheets.findIndex((sheet) => sheet.name === 'JULY 2025')
		);
	});

	it('writes nothing at all for a month with no mapped days', async () => {
		const fixture = await loadTemplate();
		const written = await writeAttendanceToWorkbook({
			target: { sourcePath: fixture.path, sheetName: SHEET, roster: [], dates: [] },
			absentIdsFor: () => new Set(['s1'])
		});
		expect(written).toBe(0);

		const sheet = (await fixture.open()).getWorksheet(SHEET) as Worksheet;
		expect(cellText(sheet.getCell('R32'))).toBe(SF2_ABSENT_MARK);
	});
});

describe('workbookXCells', () => {
	it('reports only the X marks inside the scope', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet(SHEET) as Worksheet;
		const scope: Sf2GridCell[] = [
			{ sheetName: SHEET, columnLetter: 'U', rowIndex: 8 },
			{ sheetName: SHEET, columnLetter: 'U', rowIndex: 9 }
		];
		const found = workbookXCells(sheet, scope);
		expect(found.length).toBeGreaterThan(0);
		for (const cell of found) {
			expect(cellText(sheet.getCell(`${cell.columnLetter}${cell.rowIndex}`))).toBe(SF2_ABSENT_MARK);
		}
	});

	it('reports nothing when the scope names another worksheet', async () => {
		const fixture = await loadTemplate();
		const sheet = (await fixture.open()).getWorksheet(SHEET) as Worksheet;
		expect(
			workbookXCells(sheet, [{ sheetName: 'OCTOBER 2025', columnLetter: 'U', rowIndex: 8 }])
		).toEqual([]);
	});
});

describe('the write progress phases', () => {
	it('are the four labels the modal already shows', () => {
		expect(
			writePhases({ clearMarks: [], marks: [], formulaMarks: [], staticMarks: [] }).map(
				(phase) => phase.label
			)
		).toEqual([
			'Clearing withdrawn marks',
			'Writing attendance marks',
			'Updating formulas',
			'Writing totals'
		]);
	});

	it('move once per chunk, starting before the first chunk lands', () => {
		const updates: Sf2ProgressUpdate[] = [];
		const marks = Array.from({ length: WRITE_CHUNK_SIZE * 2 + 1 }, (_, index) => ({
			sheetName: SHEET,
			address: `Z${index + 100}`,
			value: ''
		}));
		reportWriteProgress(
			(update) => updates.push(update),
			writePhases({ clearMarks: [], marks, formulaMarks: [], staticMarks: [] })
		);

		expect(updates[0].message).toBe('Preparing the workbook…');
		expect(updates).toHaveLength(4);
		expect(updates.at(-1)?.current).toBe(69);
		expect(updates.filter((update) => update.message.includes('(3/3)'))).toHaveLength(1);
	});

	it('land on 69 whatever the chunk counts are', () => {
		const updates: Sf2ProgressUpdate[] = [];
		const marks = Array.from({ length: 7 }, (_, index) => ({
			sheetName: SHEET,
			address: `Z${index + 100}`,
			value: ''
		}));
		reportWriteProgress(
			(update) => updates.push(update),
			writePhases({
				clearMarks: marks,
				marks,
				formulaMarks: marks,
				staticMarks: marks
			})
		);
		expect(updates.at(-1)?.current).toBe(69);
	});
});
