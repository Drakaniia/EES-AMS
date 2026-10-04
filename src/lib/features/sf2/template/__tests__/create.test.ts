/**
 * `createWorkbookFromTemplate`, end to end against the real bundled template.
 *
 * The whole point of these two assertions is the pair: a workbook written from the
 * template must carry none of the template's sample marks, and the split that runs
 * right after must therefore record all twelve months. Either half alone passes on
 * the broken build - the marks are invisible until the split refuses to write.
 */

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { readFile } from 'node:fs/promises';
import { useDriver } from '$lib/db';
import { NodeSqlDriver } from '$lib/db/node-driver';
import { migrate } from '$lib/db/migrations';
import { openWorkbook } from '$lib/features/excel/workbook';
import { MemoryFileSystem, useFileSystem } from '$lib/platform/fs';
import { SF2_ABSENT_MARK } from '$lib/features/sf2/logic';
import { useSf2WorkbookDir } from '$lib/features/sf2/workbook-files';
import { mergeWorkbooks } from '$lib/features/sf2/month/merge';
import { TEMPLATE_PATH } from '$lib/features/excel/__tests__/template-fixture';
import { createWorkbookFromTemplate } from '../create';

const templateBytes = new Uint8Array(await readFile(TEMPLATE_PATH));
globalThis.fetch = (async () => new Response(templateBytes, { status: 200 })) as typeof fetch;

const CLASS_ID = 'class-3-matapat';

let driver: NodeSqlDriver;

beforeEach(async () => {
	driver = new NodeSqlDriver();
	await migrate(driver);
	useDriver(driver);
	useFileSystem(new MemoryFileSystem());
	await driver.execute(
		`INSERT INTO classes (id, name, day_start, day_end, late_after, created_at)
		 VALUES (?, '3 - MATAPAT', '08:00', '15:00', '08:45', 1)`,
		[CLASS_ID]
	);
	await driver.execute(
		`INSERT INTO students (id, name, class_id, created_at, gender) VALUES
		 ('s1', 'DELA CRUZ, JUAN', ?, 1, 'male'),
		 ('s2', 'SANTOS, MARIA', ?, 1, 'female')`,
		[CLASS_ID, CLASS_ID]
	);
});

afterEach(async () => {
	useDriver(null);
	await driver.close();
});

/** Every `X` the file holds, as `SHEET!ADDRESS`. */
async function absentMarksIn(path: string): Promise<string[]> {
	const found: string[] = [];
	const workbook = await openWorkbook(path);
	for (const sheet of workbook.worksheets) {
		sheet.eachRow((row) => {
			row.eachCell((cell) => {
				if (
					String(cell.value ?? '')
						.trim()
						.toUpperCase() === SF2_ABSENT_MARK
				) {
					found.push(`${sheet.name}!${cell.address}`);
				}
			});
		});
	}
	return found;
}

describe('a workbook created from the bundled template', () => {
	it('holds none of the template sample marks, so the split that follows records all twelve', async () => {
		useSf2WorkbookDir('/workbooks');
		const summary = await createWorkbookFromTemplate({
			classId: CLASS_ID,
			schoolId: '132839',
			schoolName: 'ESPIRITU ELEMENTARY SCHOOL',
			schoolYear: '2026-2027',
			reportMonth: 'OCTOBER',
			gradeLevel: '3',
			section: 'MATAPAT',
			adviserName: 'DELA CRUZ, JUAN',
			schoolHeadName: 'SANTOS, MARIA',
			firstSchoolDay: 1,
			learnerNames: ['DELA CRUZ, JUAN', 'SANTOS, MARIA']
		});

		// The template ships 40-odd sample `X` marks of its own.
		expect(await absentMarksIn(summary.sourcePath)).toEqual([]);

		const outcome = await mergeWorkbooks();
		expect(outcome.needsAttentionCount).toBe(0);
		const months = await driver.query<{ report_month: string }>(
			'SELECT report_month FROM sf2_month_templates ORDER BY report_year, report_month'
		);
		expect(months).toHaveLength(12);
		expect(months.map((row) => row.report_month)).toContain('OCTOBER');
	});
});
