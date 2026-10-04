import { readFile, writeFile } from 'node:fs/promises';
import { describe, it } from 'vitest';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import {
	columnNumber,
	getCellText,
	openWorkbook,
	saveWorkbookAtomic,
	writableDayColumns
} from '$lib/features/excel/workbook';
import {
	applyMarks,
	clearRange,
	writeFormulaMarks,
	writeMarksForce
} from '$lib/features/excel/marks';
import { cellAddress, cellText } from '$lib/features/excel/workbook';
import {
	copyFormSheet,
	donorFormSheet,
	emptyMonthSheet,
	growRosterRows,
	prepareMonthSheet,
	totalRowsOnSheet,
	weekdaySlots,
	writeTotalLabels
} from '../workbook-sheets';
import type { Sf2CellMark } from '$lib/features/excel/types';

const LEGACY_PATH =
	'C:/Users/Qwenzy/Documents/EES-AMS/workbooks/_legacy/SF2-GRADE-3-MATAPAT-a253179c.xlsx';
const TMP = 'C:/Users/Qwenzy/AppData/Local/Temp/opencode/';

function learners(): { rowIndex: number; name: string; itemNumber: number; genderBlock: string }[] {
	const out: { rowIndex: number; name: string; itemNumber: number; genderBlock: string }[] = [];
	for (let i = 0; i < 15; i += 1)
		out.push({ rowIndex: 8 + i, name: `BOY ${i}, JUAN`, itemNumber: i + 1, genderBlock: 'MALE' });
	for (let i = 0; i < 10; i += 1)
		out.push({
			rowIndex: 30 + i,
			name: `GIRL ${i}, ANA`,
			itemNumber: i + 1,
			genderBlock: 'FEMALE'
		});
	return out;
}

describe('which build step loses the merges', () => {
	it('counts merges after every step', async () => {
		const PATH = '/w.xlsx';
		const fileSystem = new MemoryFileSystem();
		useFileSystem(fileSystem);
		await fileSystem.writeFileAtomic(PATH, new Uint8Array(await readFile(LEGACY_PATH)));
		const workbook = await openWorkbook(PATH);
		const donor = donorFormSheet(workbook);
		const prepared = prepareMonthSheet(workbook, 9, 2026);
		const sheetName = prepared.name;
		const sheet = workbook.getWorksheet(sheetName)!;

		const report = (step: string) =>
			console.log(`${String(sheet.model.merges.length).padStart(4)}  after ${step}`);

		report('prepareMonthSheet');
		copyFormSheet(donor, sheet);
		report('copyFormSheet');

		const current = totalRowsOnSheet(sheet);
		console.log('     totalRowsOnSheet:', JSON.stringify(current));
		growRosterRows(sheet, 6, 5, current.maleTotalRow, current.femaleTotalRow);
		report('growRosterRows(6,5)');

		emptyMonthSheet(sheet, 29, 49);
		report('emptyMonthSheet(29,49)');

		writeTotalLabels(sheet, 29, 49, 50);
		report('writeTotalLabels');

		const slots = weekdaySlots(sheet);
		console.log('     weekdaySlots:', slots.length);
		const marks: Sf2CellMark[] = [];
		for (const learner of learners()) {
			marks.push(
				{ sheetName, address: cellAddress(learner.rowIndex, 1), value: String(learner.itemNumber) },
				{ sheetName, address: cellAddress(learner.rowIndex, 3), value: learner.name }
			);
		}
		applyMarks(workbook, marks, { textFormat: true });
		report('applyMarks(roster)');

		for (const learner of learners()) {
			marks.length = 0;
			marks.push(
				{
					sheetName,
					address: cellAddress(learner.rowIndex, 39),
					formula: `COUNTIF(F${learner.rowIndex}:AL${learner.rowIndex},"X")`,
					cachedValue: 0
				},
				{
					sheetName,
					address: cellAddress(learner.rowIndex, 41),
					formula: `$AW$5-AM${learner.rowIndex}`,
					cachedValue: 22
				}
			);
			writeFormulaMarks(workbook, marks);
		}
		report('writeFormulaMarks');

		marks.length = 0;
		marks.push({ sheetName, address: cellAddress(29, 3), value: 'MALE TOTAL' });
		writeMarksForce(workbook, marks);
		report('writeMarksForce');

		void writableDayColumns;
		void columnNumber;
		void cellText;
		void getCellText;
		void clearRange;

		await saveWorkbookAtomic(workbook, PATH);
		report('saveWorkbookAtomic (in memory)');
		await writeFile(`${TMP}steps.xlsx`, Buffer.from(await fileSystem.readFile(PATH)));
	});
});
