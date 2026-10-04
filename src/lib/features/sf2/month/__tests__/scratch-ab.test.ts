import { readFile, writeFile } from 'node:fs/promises';
import { describe, it } from 'vitest';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { openWorkbook, saveWorkbookAtomic } from '$lib/features/excel/workbook';
import {
	copyFormSheet,
	donorFormSheet,
	growRosterRows,
	prepareMonthSheet
} from '../workbook-sheets';
import { materialiseSharedFormulas } from '$lib/features/excel/workbook';

const LEGACY_PATH =
	'C:/Users/Qwenzy/Documents/EES-AMS/workbooks/_legacy/SF2-GRADE-3-MATAPAT-a253179c.xlsx';
const TMP = 'C:/Users/Qwenzy/AppData/Local/Temp/opencode/';

/** The pre-fix body: splice rows, trusting ExcelJS to carry the merges. */
function growOld(
	sheet: Parameters<typeof growRosterRows>[0],
	extraMale: number,
	extraFemale: number,
	maleTotalRow: number,
	femaleTotalRow: number
) {
	if (extraMale <= 0 && extraFemale <= 0) return;
	materialiseSharedFormulas(sheet);
	for (let i = 0; i < extraMale; i += 1) sheet.spliceRows(maleTotalRow, 0, []);
	for (let i = 0; i < extraFemale; i += 1) sheet.spliceRows(femaleTotalRow + extraMale, 0, []);
}

async function build(label: string, grow: typeof growOld | typeof growRosterRows) {
	const PATH = '/w.xlsx';
	const fileSystem = new MemoryFileSystem();
	useFileSystem(fileSystem);
	await fileSystem.writeFileAtomic(PATH, new Uint8Array(await readFile(LEGACY_PATH)));
	const workbook = await openWorkbook(PATH);
	const donor = donorFormSheet(workbook);
	const prepared = prepareMonthSheet(workbook, 9, 2026);
	const sheet = workbook.getWorksheet(prepared.name)!;
	copyFormSheet(donor, sheet);
	grow(sheet, 6, 5, 23, 38);
	await saveWorkbookAtomic(workbook, PATH);
	await writeFile(`${TMP}ab-${label}.xlsx`, Buffer.from(await fileSystem.readFile(PATH)));
	console.log(`${label}: in-memory model.merges = ${sheet.model.merges.length}`);
}

describe('A/B the growth fix', () => {
	it('writes both', async () => {
		await build('old', growOld);
		await build('fixed', growRosterRows);
	});
});
