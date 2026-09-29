/**
 * The passive workbook status line (spec §12.2).
 *
 * The line is the whole of what replaced the amber "no X marks in the app for
 * this month" recovery panel, so its job is narrow and worth pinning down: say
 * what was counted, when, and whether the two sides agree — and say nothing else.
 * In particular it must never imply that anything is waiting on the user, because
 * the recovery it used to prompt for now happens unattended at startup (§8.2).
 *
 * The three states are kept distinct on purpose. "Never counted" and "counted,
 * and it holds nothing" look identical if the zero is dropped, and a teacher
 * cannot tell "the app has no idea" from "the app looked and there was nothing
 * there" — which is the difference between waiting and moving on.
 */
import { describe, it, expect } from 'vitest';

import { workbookStatusLine, formatCheckedAt } from './report-state.svelte';

describe('workbookStatusLine', () => {
	it('reports the checked time, the workbook count and the app count when they agree', () => {
		const line = workbookStatusLine({
			scannedAt: 1_762_000_000,
			workbookXCount: 12,
			appXCount: 12
		});

		expect(line.text).toBe(
			`Last checked ${formatCheckedAt(1_762_000_000)} · workbook has 12 X · app has 12`
		);
		expect(line.agrees).toBe(true);
	});

	it('says so plainly when the two counts disagree', () => {
		// The state §9.1 exists for: the guard refuses every write until the
		// workbook and the database agree, so this is worth flagging rather than
		// rendering as two numbers that look like each other.
		const line = workbookStatusLine({
			scannedAt: 1_762_000_000,
			workbookXCount: 12,
			appXCount: 9
		});

		expect(line.text).toContain('workbook has 12 X');
		expect(line.text).toContain('app has 9');
		expect(line.agrees).toBe(false);
	});

	it('does not conflate a workbook that was never counted with one that holds nothing', () => {
		// Zero X in the file and zero X in the app is a measured agreement.
		// Unmeasured is a third state, and the line must not dress it up as one.
		const unmeasured = workbookStatusLine({
			scannedAt: null,
			workbookXCount: null,
			appXCount: 0
		});
		const measuredEmpty = workbookStatusLine({
			scannedAt: 1_762_000_000,
			workbookXCount: 0,
			appXCount: 0
		});

		expect(unmeasured.text).toContain('has not been checked yet');
		expect(unmeasured.text).not.toContain('workbook has 0 X');
		expect(unmeasured.agrees).toBeNull();

		expect(measuredEmpty.text).toContain('workbook has 0 X');
		expect(measuredEmpty.agrees).toBe(true);
	});

	it('never asks the reader to do anything', () => {
		// The panel this replaced ended in a button. Nothing here may.
		for (const counts of [
			{ scannedAt: null, workbookXCount: null, appXCount: 0 },
			{ scannedAt: null, workbookXCount: null, appXCount: 12 },
			{ scannedAt: 1, workbookXCount: 12, appXCount: 0 },
			{ scannedAt: 1, workbookXCount: 0, appXCount: 12 },
			{ scannedAt: 1, workbookXCount: 12, appXCount: 12 }
		]) {
			const { text } = workbookStatusLine(counts);
			expect(text.toLowerCase()).not.toContain('import');
			expect(text.toLowerCase()).not.toContain('click');
			expect(text.toLowerCase()).not.toContain('recover');
			expect(text.toLowerCase()).not.toContain('repair');
			expect(text).not.toContain('button');
		}
	});
});

describe('formatCheckedAt', () => {
	it('renders a real time for a real timestamp', () => {
		const rendered = formatCheckedAt(1_762_000_000);
		expect(rendered).not.toBe('never');
		// A month and a clock time, not a bare number. The exact wording is the
		// runtime locale's business, so this checks the shape rather than the text.
		expect(rendered).toContain('Nov');
		expect(rendered).toMatch(/\d{1,2}:\d{2}/);
	});

	it('says never rather than a date it does not have', () => {
		// `workbookXCount` and `workbookScannedAt` are written together by the
		// backend, but "Last checked never" is a worse sentence than one word.
		expect(formatCheckedAt(null)).toBe('never');
		expect(formatCheckedAt(0)).toBe('never');
	});
});
