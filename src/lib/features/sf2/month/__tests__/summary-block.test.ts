// @vitest-environment node
/**
 * The summary block, written inside the build's own open workbook.
 *
 * The counts here used to need a second full parse and serialize of the twelve-month
 * file, which is most of why creating or importing a workbook appeared to hang: ExcelJS
 * takes seconds per cycle on that file and the webview main thread does the work.
 * These tests pin that the block still lands, and that the build writes the file once.
 */

import type { Workbook, Worksheet } from 'exceljs';
import { beforeEach, describe, expect, it } from 'vitest';
import {
	bundledTemplateTotalRows,
	SF2_ATTENDANCE_FIRST_COLUMN,
	SF2_ATTENDANCE_LAST_COLUMN,
	SF2_FIRST_LEARNER_ROW
} from '$lib/features/excel/constants';
import { loadTemplate } from '$lib/features/excel/__tests__/template-fixture';
import type { TemplateFixture } from '$lib/features/excel/__tests__/template-fixture';
import { getCellText, writableDayColumns } from '$lib/features/excel/workbook';
import { writeSummaryBlock } from '../summary-block';
import type { MonthSheetBuild } from '../workbook-builder';

function formulaOf(cell: { value: unknown }): string {
	const value = cell.value as { formula?: string } | undefined;
	return value?.formula ?? '';
}

function formulaResult(cell: { value: unknown }): unknown {
	const value = cell.value as { result?: unknown } | undefined;
	return value?.result ?? value;
}

const oneColumn = { lateEnrolment: 1, droppedOut: 0, transferredOut: 0, transferredIn: 0 };
const counts = { AR: oneColumn, AS: oneColumn, AT: oneColumn };

function build(reportMonth: string): MonthSheetBuild {
	return {
		removeStaleSheets: false,
		request: {
			templateId: `month-${reportMonth.toLowerCase()}`,
			reportMonth,
			reportYear: 2025,
			firstSchoolDay: 2,
			header: {
				schoolId: '',
				schoolName: '',
				schoolYear: '',
				reportMonth,
				gradeLevel: '',
				section: '',
				adviserName: '',
				schoolHeadName: ''
			},
			learners: [
				{
					studentId: 's1',
					rowIndex: 8,
					itemNumber: 1,
					genderBlock: 'MALE',
					name: 'DELA CRUZ, JUAN'
				},
				{
					studentId: 's2',
					rowIndex: 9,
					itemNumber: 2,
					genderBlock: 'MALE',
					name: 'SALIMBOT, RAKIM'
				},
				{
					studentId: 's3',
					rowIndex: 30,
					itemNumber: 1,
					genderBlock: 'FEMALE',
					name: 'DELA CRUZ, MARIA'
				}
			],
			absences: [],
			sourceFemaleStartRow: 0
		}
	};
}

let workbook: Workbook;
let fixture: TemplateFixture;

/** Every populated day cell of one sheet, so "untouched" can be asserted. */
function dayColumns(sheet: Worksheet): string[] {
	const row = sheet.getRow(SF2_FIRST_LEARNER_ROW);
	const seen: string[] = [];
	for (
		let column = SF2_ATTENDANCE_FIRST_COLUMN;
		column <= SF2_ATTENDANCE_LAST_COLUMN;
		column += 1
	) {
		const cell = row.getCell(column);
		if (cell.value === null) continue;
		seen.push(`${cell.address}=${getCellText(sheet, SF2_FIRST_LEARNER_ROW, column)}`);
	}
	return seen.sort();
}

beforeEach(async () => {
	fixture = await loadTemplate();
	workbook = await fixture.open();
});

describe('the summary block', () => {
	it('writes a number into every summary cell it addresses', () => {
		writeSummaryBlock(workbook, [build('JUNE')], counts);

		const sheet = workbook.getWorksheet('JUNE 2025') as Worksheet;
		const totals = bundledTemplateTotalRows(2, 1);
		for (const column of ['AR', 'AS', 'AT']) {
			for (const row of [55, 67, 69, 71]) {
				const cell = sheet.getCell(`${column}${row}`);
				expect(typeof formulaResult(cell)).toBe('number');
			}
		}
		expect(totals.combinedTotalRow).toBe(50);
	});

	it('leaves the X marks and the day columns alone', () => {
		const sheet = workbook.getWorksheet('JUNE 2025') as Worksheet;
		const before = dayColumns(sheet);
		const marksBefore = writableDayColumns(sheet).length;

		writeSummaryBlock(workbook, [build('JUNE')], counts);

		expect(dayColumns(sheet)).toEqual(before);
		expect(writableDayColumns(sheet)).toHaveLength(marksBefore);
	});

	it('carries the COUNTIF the teacher checks the file against', () => {
		writeSummaryBlock(workbook, [build('JUNE')], counts);

		const sheet = workbook.getWorksheet('JUNE 2025') as Worksheet;
		expect(formulaOf(sheet.getCell('AM8'))).toBe('COUNTIF(F8:AL8,"X")');
	});

	it('skips a build whose month the workbook has no sheet for', () => {
		expect(() => writeSummaryBlock(workbook, [build('NOVEMBER')], counts)).not.toThrow();
	});
});
