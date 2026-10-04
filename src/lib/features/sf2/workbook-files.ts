/**
 * Where SF2 workbooks live and what they are called — the port of
 * `src-tauri/src/sf2/workbook_files.rs`.
 *
 * Two things changed with the port and both are deliberate. The files moved out of
 * the app data directory into `Documents\EES-AMS\workbooks\` so a teacher can find
 * them (migration spec D13), and the extension is `.xlsx` because ExcelJS writes
 * `.xlsx` and the school accepts it (D3).
 */

import { documentDir } from '@tauri-apps/api/path';
import { internal } from '$lib/db';
import { getFileSystem } from '$lib/platform/fs';
import bundledTemplateUrl from '../../../../src-tauri/resources/sf2/TEMPLATE_AUTOMATED_SF2.xlsx?url';
import { monthSheetName, sf2MonthName, sf2MonthNumber } from './calendar';
import { reportYearForSchoolMonth } from './first-school-day';
import { sanitizeFilePart } from './naming';
import type { Sf2WorkbookAnalysis } from './calendar';

/** The bundled DepEd template, as a URL the app can fetch its bytes from. */
export const BUNDLED_TEMPLATE_URL: string = bundledTemplateUrl;

/** The folder under Documents that holds everything the app writes (spec D13). */
export const SF2_ROOT_FOLDER = 'EES-AMS';

export const SF2_WORKBOOKS_FOLDER = 'workbooks';

/**
 * Folder inside the workbooks directory that keeps the pre-split per-class workbook.
 *
 * The file is copied here by the merge and is never modified or deleted: it stays the
 * last authoritative copy of the original workbook, marks and all. A backup snapshot
 * walks the workbooks tree recursively, so this folder is included without any
 * special case.
 */
export const LEGACY_WORKBOOK_DIR = '_legacy';

/**
 * The twelve calendar months of a school year.
 *
 * Calendar order, not school-year order: the pair of `(month, year)` is sorted
 * below, because the year is what wraps.
 */
const SCHOOL_YEAR_MONTHS = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

/** The class a workbook belongs to, and the school year it is dated in. */
type WorkbookIdentity = { gradeLevel: string; section: string; templateId: string };

// ── Where the files live ─────────────────────────────────────────────────────

let injectedWorkbookDir: string | null = null;

/** Test seam, mirroring `useFileSystem()`. Pass `null` for the real Documents path. */
export function useSf2WorkbookDir(path: string | null): void {
	injectedWorkbookDir = path === null ? null : normalizePath(path);
}

/**
 * The one workbook directory, created if it is not there yet.
 *
 * Every workbook path in the app is derived from this and nothing else, because the
 * old Rust version had two ways to reach the same directory and the app data path
 * is one the teacher cannot open.
 */
export async function getSf2WorkbookDir(): Promise<string> {
	if (injectedWorkbookDir !== null) return injectedWorkbookDir;

	const documents = await documentDir();
	const dir = normalizePath(`${documents}/${SF2_ROOT_FOLDER}/${SF2_WORKBOOKS_FOLDER}`);
	await getFileSystem().mkdirp(dir);
	return dir;
}

/** Where the pre-split per-class workbook is kept once the split has run. */
export async function sf2LegacyWorkbookDir(): Promise<string> {
	return `${await getSf2WorkbookDir()}/${LEGACY_WORKBOOK_DIR}`;
}

function normalizePath(path: string): string {
	return path.replace(/\\/g, '/').replace(/\/+$/, '');
}

function joinPath(directory: string, name: string): string {
	return `${normalizePath(directory)}/${name}`;
}

// ── What the files are called ────────────────────────────────────────────────

function sanitizedOr(value: string, fallback: string): string {
	const sanitized = sanitizeFilePart(value);
	return sanitized === '' ? fallback : sanitized;
}

/**
 * Uppercase, canonical month name.
 *
 * `Sept.`, `sept` and `SEPTEMBER` all become `SEPTEMBER`; anything unrecognised is
 * uppercased as typed, and an empty month keeps the same `MONTH` fallback the export
 * path has always used.
 */
export function canonicalMonthName(reportMonth: string): string {
	const month = sf2MonthNumber(reportMonth);
	const canonical = month === undefined ? reportMonth.trim().toUpperCase() : sf2MonthName(month);
	return canonical === '' ? 'MONTH' : canonical;
}

/**
 * The one worksheet a month lives on inside the one workbook: `SEPTEMBER 2026`.
 *
 * The same string a `sf2_date_mappings.sheet_name` holds, so a write addressed by a
 * stored column finds the worksheet it was recorded against.
 */
export function monthWorkbookSheetName(reportMonth: string, reportYear: number): string {
	return monthSheetName(sf2MonthNumber(reportMonth) ?? 0, reportYear);
}

/**
 * The file name of a **retired** per-month workbook: `SF2-SEPTEMBER-2026.xlsx`.
 *
 * Under spec section 0 A1 a month has no file of its own. These helpers survive for
 * two honest reasons and no others: the merge looks for such a file so it can fold
 * anything unique in it into the single workbook and then leaves it alone, and the
 * preview still reports whether one is on disk so a teacher whose install has one is
 * told about it rather than having it vanish. Nothing creates one any more.
 */
export function monthWorkbookFileName(reportMonth: string, reportYear: number): string {
	return `SF2-${canonicalMonthName(reportMonth)}-${reportYear}.xlsx`;
}

/** Where a retired per-month workbook would be. May not exist; never created here. */
export function monthWorkbookPath(
	workbookDir: string,
	reportMonth: string,
	reportYear: number
): string {
	return joinPath(workbookDir, monthWorkbookFileName(reportMonth, reportYear));
}

/**
 * The pre-split per-class file name: `SF2-GRADE-3-MATAPAT-3b635890.xlsx`.
 *
 * Still resolved after the split, because the split job and any pre-split install
 * have to find the original workbook.
 */
export function legacyWorkbookFileName(
	templateId: string,
	gradeLevel: string,
	section: string
): string {
	return `SF2-${sanitizedOr(gradeLevel, 'GRADE')}-${sanitizedOr(section, 'SECTION')}-${templateId.slice(0, 8)}.xlsx`;
}

/**
 * The one workbook for a class.
 *
 * The same name the pre-split per-class file had, which is the point: the file on
 * the user's disk today already has this name, and the merge rebuilds *that* file
 * into the twelve-sheet workbook rather than writing a new one beside it. The
 * pre-merge content is preserved first, in `_legacy/`.
 */
export function singleWorkbookFileName(identity: WorkbookIdentity): string {
	return legacyWorkbookFileName(identity.templateId, identity.gradeLevel, identity.section);
}

/** Where the one workbook lives. */
export function singleWorkbookPath(workbookDir: string, identity: WorkbookIdentity): string {
	return joinPath(workbookDir, singleWorkbookFileName(identity));
}

/**
 * Every workbook file on disk that names one class: the working copy plus any
 * orphan a re-import, a retried split, or a manual copy left behind.
 *
 * Two files under one class stem is how "Open SF2" opens the wrong document:
 * the month rows name one path while a stale copy sits beside it. The open
 * path refuses to guess between them. Directories (`_legacy/`,
 * `import-staging/`) and atomic-write temp files (`*.tmp`) are never
 * workbooks.
 */
export async function classWorkbookFiles(
	workbookDir: string,
	gradeLevel: string,
	section: string
): Promise<string[]> {
	const stem = `SF2-${sanitizedOr(gradeLevel, 'GRADE')}-${sanitizedOr(section, 'SECTION')}-`;
	const names = await getFileSystem().readDir(workbookDir);
	return names
		.filter((name) => !name.endsWith('/'))
		.filter((name) => name.startsWith(stem) && name.toLowerCase().endsWith('.xlsx'))
		.map((name) => joinPath(workbookDir, name))
		.sort();
}

/**
 * The export file name: `SF2-GRADE-3-MATAPAT-JUNE-generated.xlsx`.
 *
 * Falls back to the current calendar month when the template's own report month is
 * not a month name, so a template saved in a hurry still exports under a name the
 * teacher can recognise.
 */
export function exportWorkbookFileName(
	gradeLevel: string,
	section: string,
	reportMonth: string,
	currentMonth: number
): string {
	const month = sf2MonthNumber(reportMonth) ?? currentMonth;
	const name = sf2MonthName(month);
	return `SF2-${sanitizedOr(gradeLevel, 'GRADE')}-${sanitizedOr(section, 'SECTION')}-${name === '' ? 'MONTH' : name}-generated.xlsx`;
}

/**
 * The twelve `(month name, calendar year)` pairs of a school year, in the order the
 * school year runs them: SEPTEMBER -> AUGUST.
 *
 * `fallbackYear` is used only when the label holds no four-digit year. Ordered by
 * school-year position, not by month number: the months are 1..12 but the year wraps
 * at September, so a plain month order would list January first and read as a bug to
 * the teacher.
 */
export function schoolYearMonthFiles(
	schoolYear: string,
	fallbackYear: number
): { month: string; reportYear: number }[] {
	return SCHOOL_YEAR_MONTHS.map((monthNumber) => ({
		monthNumber,
		month: sf2MonthName(monthNumber),
		reportYear: reportYearForSchoolMonth(schoolYear, monthNumber, fallbackYear)
	}))
		.sort(
			(left, right) => left.reportYear - right.reportYear || left.monthNumber - right.monthNumber
		)
		.map(({ month, reportYear }) => ({ month, reportYear }));
}

// ── Writing the template ─────────────────────────────────────────────────────

/**
 * The bytes of the bundled DepEd template.
 *
 * Rust embedded the file with `include_bytes!`, which has no TypeScript equivalent;
 * Vite's asset URL is the same idea — the template ships inside the app, and a
 * missing one is a build failure rather than a runtime surprise.
 */
export async function readBundledTemplate(): Promise<Uint8Array> {
	const response = await fetch(bundledTemplateUrl);
	if (!response.ok) {
		throw internal(`failed to read the bundled SF2 template (${bundledTemplateUrl})`);
	}
	return new Uint8Array(await response.arrayBuffer());
}

/**
 * Write a fresh copy of the bundled template to `path`.
 *
 * A month worksheet starts as a copy of the template and then loses everything the
 * template shipped in it — its sample roster and its sample `X` marks — before the
 * class's own roster and absences are written.
 */
export async function writeBundledTemplateTo(path: string): Promise<void> {
	await getFileSystem().writeFileAtomic(path, await readBundledTemplate());
}

/**
 * Write the working copy of the bundled template into the workbook directory.
 *
 * A month starts from the template rather than from the previous month's file,
 * because cloning that file would put one month's `X` marks in another.
 */
export async function writeBundledTemplateToDir(identity: WorkbookIdentity): Promise<string> {
	const path = singleWorkbookPath(await getSf2WorkbookDir(), identity);
	await writeBundledTemplateTo(path);
	return path;
}

// ── The layout fingerprint ───────────────────────────────────────────────────

/**
 * A stable hash of everything a write path addresses: the sheets and their used
 * ranges, every learner row, and every date column.
 *
 * It answers "is this still the workbook I described?" without re-reading the file.
 * The Rust used a hand-rolled FNV-1a over the same field order, and the field order
 * is kept exactly: a fingerprint stored in `sf2_templates.layout_fingerprint` has to
 * come out the same on a re-import.
 */
export function layoutFingerprint(analysis: Sf2WorkbookAnalysis): string {
	const bytes: number[] = [];
	const push = (text: string): void => {
		for (const byte of new TextEncoder().encode(text)) bytes.push(byte);
	};

	for (const sheet of analysis.sheets) {
		push(sheet.name);
		push(sheet.usedRange);
	}
	for (const learner of analysis.learners) {
		push(learner.name);
		// Rust appended `row_index.to_le_bytes()`, i.e. four little-endian bytes.
		bytes.push(learner.rowIndex & 0xff, (learner.rowIndex >> 8) & 0xff);
	}
	for (const date of analysis.dates) {
		push(date.date);
		push(date.sheetName);
		push(date.columnLetter);
	}

	return hashBytes(new Uint8Array(bytes));
}

/**
 * FNV-1a, 64-bit, as a 16-character hex string.
 *
 * `BigInt` because the multiply overflows 32 bits; `0xcbf29ce484222325n` is the
 * offset basis and the value 0 is treated as "nothing hashed yet", which is what the
 * Rust `Default` did.
 */
export function hashBytes(bytes: Uint8Array): string {
	const OFFSET = 0xcbf29ce484222325n;
	const PRIME = 0x100000001b3n;
	const MASK = 0xffffffffffffffffn;

	let hash = OFFSET;
	for (const byte of bytes) {
		hash = ((hash ^ BigInt(byte)) * PRIME) & MASK;
	}
	return hash.toString(16).padStart(16, '0');
}
