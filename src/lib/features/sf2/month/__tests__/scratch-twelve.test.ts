import { readFile, writeFile } from 'node:fs/promises';
import { describe, it } from 'vitest';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { openWorkbook } from '$lib/features/excel/workbook';
import type { MonthLearnerWrite } from '../workbook-builder';
import { buildSchoolYearWorkbook } from '../workbook-builder';

const LEGACY_PATH =
	'C:/Users/Qwenzy/Documents/EES-AMS/workbooks/_legacy/SF2-GRADE-3-MATAPAT-a253179c.xlsx';
const TMP = 'C:/Users/Qwenzy/AppData/Local/Temp/opencode/';
const MONTHS = [
	['SEPTEMBER', 2026],
	['OCTOBER', 2026],
	['NOVEMBER', 2026],
	['DECEMBER', 2026],
	['JANUARY', 2027],
	['FEBRUARY', 2027],
	['MARCH', 2027],
	['APRIL', 2027],
	['MAY', 2027],
	['JUNE', 2027],
	['JULY', 2027],
	['AUGUST', 2027]
] as const;

function roster(maleCount: number, femaleCount: number, femaleStartRow = 30): MonthLearnerWrite[] {
	const out: MonthLearnerWrite[] = [];
	for (let i = 0; i < maleCount; i += 1)
		out.push({
			studentId: `m${i}`,
			rowIndex: 8 + i,
			name: `BOY ${i}, JUAN`,
			itemNumber: i + 1,
			genderBlock: 'MALE'
		});
	for (let i = 0; i < femaleCount; i += 1)
		out.push({
			studentId: `f${i}`,
			rowIndex: femaleStartRow + i,
			name: `GIRL ${i}, ANA`,
			itemNumber: i + 1,
			genderBlock: 'FEMALE'
		});
	return out;
}

function request(reportMonth: string, reportYear: number, learners: MonthLearnerWrite[]) {
	return {
		templateId: 't1',
		reportMonth,
		reportYear,
		firstSchoolDay: 1,
		header: {
			schoolId: '132839',
			schoolName: 'ESPIRITU ELEMENTARY SCHOOL',
			schoolYear: '2026-2027',
			reportMonth,
			gradeLevel: 'GRADE 3',
			section: 'MATAPAT',
			adviserName: 'DELA CRUZ, JUAN',
			schoolHeadName: 'SANTOS, MARIA'
		},
		learners,
		absences: [],
		sourceFemaleStartRow: 30
	};
}

describe('twelve months in one call', () => {
	it('counts merges on every sheet', async () => {
		const PATH = '/w.xlsx';
		const fileSystem = new MemoryFileSystem();
		useFileSystem(fileSystem);
		await fileSystem.writeFileAtomic(PATH, new Uint8Array(await readFile(LEGACY_PATH)));

		const learners = roster(15, 10);
		const report = await buildSchoolYearWorkbook(
			PATH,
			MONTHS.map(([m, y]) => ({ request: request(m, y, learners), removeStaleSheets: true }))
		);
		console.log('verification:', JSON.stringify(report.verification));
		await writeFile(`${TMP}twelve.xlsx`, Buffer.from(await fileSystem.readFile(PATH)));

		const wb = await openWorkbook(PATH);
		for (const ws of wb.worksheets) console.log(`  ${ws.name}: merges=${ws.model.merges.length}`);
	});
});
