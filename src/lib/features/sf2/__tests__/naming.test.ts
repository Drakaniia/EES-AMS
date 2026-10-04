/**
 * Naming rules: what a class is called and what a file on disk is called.
 *
 * These are the two functions that decide whether the app can find its own work
 * again — a class is matched by name and a workbook by file name — so the cases
 * below are the ones where a blank cell or an odd character would otherwise
 * produce a name that matches nothing.
 */
import { describe, expect, it } from 'vitest';
import { className, sanitizeFilePart } from '../naming';
import {
	canonicalMonthName,
	exportWorkbookFileName,
	legacyWorkbookFileName,
	monthWorkbookFileName,
	monthWorkbookSheetName,
	schoolYearMonthFiles,
	singleWorkbookFileName,
	singleWorkbookPath,
	useSf2WorkbookDir,
	getSf2WorkbookDir
} from '../workbook-files';

describe('className', () => {
	it('joins both halves', () => {
		expect(className('3', 'MATAPAT')).toBe('3 - MATAPAT');
	});

	it('trims before joining', () => {
		expect(className('  3 ', ' MATAPAT  ')).toBe('3 - MATAPAT');
	});

	it('drops a blank half rather than leaving a dangling separator', () => {
		expect(className('3', '   ')).toBe('3');
		expect(className('', 'MATAPAT')).toBe('MATAPAT');
	});

	it('names the class when the workbook says nothing at all', () => {
		expect(className('', '')).toBe('SF2 Class');
	});
});

describe('sanitizeFilePart', () => {
	it('uppercases and separates with single dashes', () => {
		expect(sanitizeFilePart('Grade 3')).toBe('GRADE-3');
	});

	it('collapses a run of separators', () => {
		expect(sanitizeFilePart('Grade   3')).toBe('GRADE-3');
		expect(sanitizeFilePart('a - b')).toBe('A-B');
	});

	it('replaces every character Windows will not take in a path', () => {
		expect(sanitizeFilePart('a/b\\c:d*e?f"g<h>i|j')).toBe('A-B-C-D-E-F-G-H-I-J');
	});

	it('empties a value with nothing usable in it', () => {
		expect(sanitizeFilePart('///')).toBe('');
	});
});

describe('file names', () => {
	it('keeps the pre-split per-class name, which is the file already on disk', () => {
		expect(legacyWorkbookFileName('3b635890-1111-2222', 'Grade 3', 'Matapat')).toBe(
			'SF2-GRADE-3-MATAPAT-3b635890.xlsx'
		);
	});

	it('falls back rather than emitting an empty segment', () => {
		expect(legacyWorkbookFileName('abcdefgh-9999', '///', '')).toBe(
			'SF2-GRADE-SECTION-abcdefgh.xlsx'
		);
	});

	it('names the one workbook the same way', () => {
		const identity = { templateId: '3b635890-1111-2222', gradeLevel: '3', section: 'A' };
		expect(singleWorkbookFileName(identity)).toBe('SF2-3-A-3b635890.xlsx');
		expect(singleWorkbookPath('C:/Documents/EES-AMS/workbooks', identity)).toBe(
			'C:/Documents/EES-AMS/workbooks/SF2-3-A-3b635890.xlsx'
		);
	});

	it('names a retired per-month file after its canonical month', () => {
		expect(monthWorkbookFileName('Sept.', 2026)).toBe('SF2-SEPTEMBER-2026.xlsx');
		expect(monthWorkbookFileName('  june ', 2025)).toBe('SF2-JUNE-2025.xlsx');
		expect(monthWorkbookFileName('smiling', 2025)).toBe('SF2-SMILING-2025.xlsx');
		expect(monthWorkbookFileName('', 2025)).toBe('SF2-MONTH-2025.xlsx');
	});

	it('names the worksheet a month lives on', () => {
		expect(monthWorkbookSheetName('SEPT.', 2026)).toBe('SEPTEMBER 2026');
	});

	it('canonicalises an unrecognised month by uppercasing it as typed', () => {
		expect(canonicalMonthName('Sept.')).toBe('SEPTEMBER');
		expect(canonicalMonthName('may')).toBe('MAY');
		expect(canonicalMonthName('Huly')).toBe('HULY');
	});

	it('falls back to the current month in an export name it cannot read', () => {
		expect(exportWorkbookFileName('3', 'A', 'SEPTEMBER', 7)).toBe(
			'SF2-3-A-SEPTEMBER-generated.xlsx'
		);
		expect(exportWorkbookFileName('3', 'A', 'smiling', 7)).toBe('SF2-3-A-JULY-generated.xlsx');
	});
});

describe('schoolYearMonthFiles', () => {
	it('runs SEPTEMBER -> AUGUST, because that is the order a school year runs in', () => {
		expect(schoolYearMonthFiles('2026-2027', 2030)).toEqual([
			{ month: 'SEPTEMBER', reportYear: 2026 },
			{ month: 'OCTOBER', reportYear: 2026 },
			{ month: 'NOVEMBER', reportYear: 2026 },
			{ month: 'DECEMBER', reportYear: 2026 },
			{ month: 'JANUARY', reportYear: 2027 },
			{ month: 'FEBRUARY', reportYear: 2027 },
			{ month: 'MARCH', reportYear: 2027 },
			{ month: 'APRIL', reportYear: 2027 },
			{ month: 'MAY', reportYear: 2027 },
			{ month: 'JUNE', reportYear: 2027 },
			{ month: 'JULY', reportYear: 2027 },
			{ month: 'AUGUST', reportYear: 2027 }
		]);
	});

	it('accepts the DepEd spelling of the same school year', () => {
		expect(schoolYearMonthFiles('2026 - 2027', 2030)[0]).toEqual({
			month: 'SEPTEMBER',
			reportYear: 2026
		});
	});

	it('falls back to the caller year for a label with no year in it', () => {
		expect(schoolYearMonthFiles('', 2030).every((entry) => entry.reportYear === 2030)).toBe(true);
	});
});

describe('the workbook directory', () => {
	it('is the one place the path is built', async () => {
		useSf2WorkbookDir('C:/Users/teacher/Documents/EES-AMS/workbooks/');
		expect(await getSf2WorkbookDir()).toBe('C:/Users/teacher/Documents/EES-AMS/workbooks');
		useSf2WorkbookDir(null);
	});
});
