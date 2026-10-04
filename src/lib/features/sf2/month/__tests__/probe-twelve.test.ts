import { readFile, writeFile } from 'node:fs/promises';
import { describe, it } from 'vitest';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { openWorkbook } from '$lib/features/excel/workbook';
import { donorFormSheet } from '../workbook-sheets';
import type { MonthLearnerWrite } from '../workbook-builder';
import { buildSchoolYearWorkbook } from '../workbook-builder';
import { TEMPLATE_PATH } from '$lib/features/excel/__tests__/template-fixture';

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

function roster(): MonthLearnerWrite[] {
	const out: MonthLearnerWrite[] = [];
	const boys = [
		'Alvarado',
		'Belacse',
		'Calimpusan',
		'Escobido',
		'Espiritu',
		'Gontinas',
		'Gorne',
		'Lunasin'
	];
	const girls = ['Mosquito', 'Oranza', 'Perdigones', 'Suarez', 'Tinio', 'Tipontipon', 'Reyes'];
	boys.forEach((n, i) =>
		out.push({
			studentId: `m${i}`,
			rowIndex: 8 + i,
			name: `${n}, Learner ${i}`,
			itemNumber: i + 1,
			genderBlock: 'MALE'
		})
	);
	girls.forEach((n, i) =>
		out.push({
			studentId: `f${i}`,
			rowIndex: 30 + i,
			name: `${n}, Learner ${i}`,
			itemNumber: i + 1,
			genderBlock: 'FEMALE'
		})
	);
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
			schoolName: 'Espiritu Elementary School',
			schoolYear: '2026 - 2027',
			reportMonth,
			gradeLevel: 'Grade 3',
			section: 'MATAPAT',
			adviserName: 'ADVISER',
			schoolHeadName: 'HEAD'
		},
		learners,
		absences: [],
		sourceFemaleStartRow: 30
	};
}

describe('probe: twelve-month build from pristine template', () => {
	it('counts merges/styles/pagesetup after build', async () => {
		const PATH = '/w.xlsx';
		const fileSystem = new MemoryFileSystem();
		useFileSystem(fileSystem);
		await fileSystem.writeFileAtomic(PATH, new Uint8Array(await readFile(TEMPLATE_PATH)));
		const before = await openWorkbook(PATH);
		console.log(
			'DONOR =',
			donorFormSheet(before).name,
			'merges =',
			donorFormSheet(before).model.merges.length
		);

		const learners = roster();
		const report = await buildSchoolYearWorkbook(
			PATH,
			MONTHS.map(([m, y]) => ({ request: request(m, y, learners), removeStaleSheets: true }))
		);
		console.log('verification:', JSON.stringify(report.verification));
		await writeFile(`${TMP}probe-twelve.xlsx`, Buffer.from(await fileSystem.readFile(PATH)));

		const wb = await openWorkbook(PATH);
		for (const ws of wb.worksheets) {
			const styledEmpty = (() => {
				let n = 0;
				for (let r = 1; r <= ws.rowCount; r++)
					for (let c = 1; c <= ws.columnCount; c++) {
						const cell = ws.getRow(r).getCell(c);
						if (cell.value === null && cell.style.border) n++;
					}
				return n;
			})();
			console.log(
				`  ${ws.name}: merges=${ws.model.merges.length} pageSetup=${JSON.stringify(ws.pageSetup.orientation)}/${JSON.stringify(ws.pageSetup.margins?.left)} borderedEmpty=${styledEmpty}`
			);
		}
		const t = wb.getWorksheet('SEPTEMBER 2026')!;
		console.log(
			'SEPT footer merges:',
			t.model.merges.filter((m: string) => parseInt(m.match(/\d+/)?.[0] ?? '0') >= 50).length
		);
		console.log(
			'SEPT AW width:',
			t.getColumn(49).width,
			'AX:',
			t.getColumn(50).width,
			'AY:',
			t.getColumn(51).width
		);
	});
});
