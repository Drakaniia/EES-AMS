/**
 * A fixture loader for the real converted template.
 *
 * The template at `src-tauri/resources/sf2/TEMPLATE_AUTOMATED_SF2.xlsx` was
 * converted from the original DepEd `.xls` by Excel itself, so its sheet names,
 * merged ranges and cell addresses are ground truth - not something a fixture
 * should invent. Tests read the real bytes; they never need Excel installed.
 */

import { readFile } from 'node:fs/promises';
import ExcelJS from 'exceljs';
import type { Workbook } from 'exceljs';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { openWorkbook, saveWorkbookAtomic } from '../workbook';

/** The bundled DepEd template, as Excel's own conversion of the original `.xls`. */
export const TEMPLATE_PATH = 'src-tauri/resources/sf2/TEMPLATE_AUTOMATED_SF2.xlsx';

/** The six worksheets the bundled template ships with. */
export const TEMPLATE_SHEETS = [
	'JUNE 2025',
	'JULY 2025',
	'AUGUST 2025',
	'SEPT. 2025',
	'OCTOBER 2025',
	'COMPLETE DAYS'
];

/** The SF2 row layout of the bundled template: 21 male slots, 19 female slots. */
export const TEMPLATE_TOTALS = {
	maleTotalRow: 29,
	femaleTotalRow: 49,
	combinedTotalRow: 50
};

/** The template's own `TOTAL NO. OF DAYS`, which the app rewrites. */
export const TEMPLATE_DAY_COUNT = 11;

/** Boys and girls the bundled template's June roster holds. */
export const TEMPLATE_ROSTER = { maleCount: 12, femaleCount: 14 };

/** Where workbooks live from now on (spec D13). */
export const WORKBOOK_PATH = '/Documents/EES-AMS/workbooks/GRADE 3 - MATAPAT.xlsx';

/** The real template, mounted on an in-memory file system. */
export type TemplateFixture = {
	fileSystem: MemoryFileSystem;
	path: string;
	open: () => Promise<Workbook>;
	save: (workbook: Workbook) => Promise<void>;
	/** Save to a temp path and reopen from it - the golden round trip. */
	roundTrip: (workbook: Workbook) => Promise<Workbook>;
};

/** Mount the real template on an in-memory file system, bound to the app's seam. */
export async function loadTemplate(): Promise<TemplateFixture> {
	const bytes = new Uint8Array(await readFile(TEMPLATE_PATH));
	const fileSystem = new MemoryFileSystem();
	useFileSystem(fileSystem);
	await fileSystem.writeFileAtomic(WORKBOOK_PATH, bytes);

	return {
		fileSystem,
		path: WORKBOOK_PATH,
		open: () => openWorkbook(WORKBOOK_PATH),
		save: (workbook) => saveWorkbookAtomic(workbook, WORKBOOK_PATH),
		roundTrip: async (workbook) => {
			await saveWorkbookAtomic(workbook, WORKBOOK_PATH);
			return openWorkbook(WORKBOOK_PATH);
		}
	};
}

/** A blank one-sheet workbook, for tests that need no template. */
export function newWorkbook(sheetName = 'JUNE 2025'): Workbook {
	const workbook = new ExcelJS.Workbook();
	workbook.addWorksheet(sheetName);
	return workbook;
}
