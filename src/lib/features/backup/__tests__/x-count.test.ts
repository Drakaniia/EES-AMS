import { describe, expect, it } from 'vitest';
import { collectWorkbooks } from '../workbooks';
import { countXmarks } from '../x-count';
import { useBackupFixture, writeWorkbook, WORKBOOKS_DIR } from './fixture';

/**
 * `xCount` is the only evidence the restore preview has that a backup's
 * workbooks and its database agree. A count that silently returned 0 for a
 * workbook full of X marks would turn the one guard that protects a teacher's
 * marks into decoration, so it is counted from a real parsed workbook rather than
 * from the bytes.
 */
const PATH = `${WORKBOOKS_DIR}/SF2-GRADE-3-MATAPAT.xlsx`;

describe('countXmarks', () => {
	const fs = useBackupFixture();

	it('counts every X in the day grid', async () => {
		await writeWorkbook(PATH, ['X', '', 'X', 'X']);
		expect(await countXmarks(PATH)).toBe(3);
	});

	it('ignores marks that are not an X, in either case', async () => {
		await writeWorkbook(PATH, ['x', '✓', '', 'X ']);
		expect(await countXmarks(PATH)).toBe(2);
	});

	it('rejects rather than reporting a count for a file it cannot read', async () => {
		await fs.writeFileAtomic(PATH, new TextEncoder().encode('this is not a workbook'));
		await expect(countXmarks(PATH)).rejects.toThrow();
	});
});

describe('the snapshot', () => {
	const fs = useBackupFixture();

	it('records xCount 0 for a workbook it cannot read, and still backs it up', async () => {
		await fs.writeFileAtomic(PATH, new TextEncoder().encode('this is not a workbook'));

		const snapshot = await collectWorkbooks();

		expect(snapshot.sourceMissing).toBe(false);
		expect(snapshot.entries).toEqual([
			{ path: 'workbooks/SF2-GRADE-3-MATAPAT.xlsx', bytes: expect.any(Number), xCount: 0 }
		]);
	});

	it('reports a missing source directory rather than an empty one', async () => {
		expect(await collectWorkbooks()).toMatchObject({ files: [], entries: [], sourceMissing: true });
	});

	it('walks sub-folders, so `_legacy/` needs no special case', async () => {
		await writeWorkbook(PATH, ['X']);
		await writeWorkbook(`${WORKBOOKS_DIR}/_legacy/SF2-GRADE-3-MATAPAT-old.xlsx`, ['X', 'X']);

		const snapshot = await collectWorkbooks();

		expect(snapshot.entries.map((entry) => entry.path)).toEqual([
			'workbooks/SF2-GRADE-3-MATAPAT.xlsx',
			'workbooks/_legacy/SF2-GRADE-3-MATAPAT-old.xlsx'
		]);
		expect(snapshot.entries.map((entry) => entry.xCount)).toEqual([1, 2]);
	});
});
