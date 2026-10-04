/**
 * The progress reporter — the shape the Rust `sf2-progress` event had, without the
 * event.
 */
import { describe, expect, it } from 'vitest';
import {
	emitSf2Progress,
	emitSf2WriteStep,
	NO_PROGRESS,
	SF2_OPEN_STEPS,
	SF2_OPEN_TOTAL,
	type Sf2ProgressUpdate
} from '../progress';

function recorder(): { updates: Sf2ProgressUpdate[]; report: (u: Sf2ProgressUpdate) => void } {
	const updates: Sf2ProgressUpdate[] = [];
	return { updates, report: (update) => updates.push(update) };
}

describe('the Open SF2 vocabulary', () => {
	it('is the ten steps the UI maps to its own strings', () => {
		expect(SF2_OPEN_TOTAL).toBe(10);
		expect(SF2_OPEN_STEPS).toHaveLength(10);
		expect(SF2_OPEN_STEPS[0]).toBe('Loading workbook details…');
		expect(SF2_OPEN_STEPS[9]).toBe('Done!');
	});

	it('reports a step on the ten-point scale, tagged `open`', () => {
		const { updates, report } = recorder();
		emitSf2Progress(report, 4);
		expect(updates).toEqual([
			{ task: 'open', current: 4, total: 10, message: 'Clearing previous marks…' }
		]);
	});

	it('lets a caller override the message, which is how the read-only path used to explain itself', () => {
		const { updates, report } = recorder();
		emitSf2Progress(report, 4, 'Opening read-only: the file could not be measured');
		expect(updates[0].message).toBe('Opening read-only: the file could not be measured');
	});

	it('reports an out-of-range step with a blank message rather than dropping it', () => {
		const { updates, report } = recorder();
		emitSf2Progress(report, 11);
		expect(updates).toHaveLength(1);
		expect(updates[0].current).toBe(11);
		expect(updates[0].message).toBe('');
	});
});

describe('the fine-grained write phase', () => {
	it('sits between 60% and 70% on a 100-point scale', () => {
		const { updates, report } = recorder();
		// Before any work: the bar must have left 60, or it looks paused.
		emitSf2WriteStep(report, 0, 10, 'Preparing the workbook…');
		// Every chunk done.
		emitSf2WriteStep(report, 10, 10, 'Writing totals (10/10)…');
		// More chunks than units asked for still cannot leave the window.
		emitSf2WriteStep(report, 99, 1, 'done');

		expect(updates.map((u) => u.current)).toEqual([61, 69, 69]);
		expect(updates.every((u) => u.total === 100)).toBe(true);
	});

	it('advances monotonically as chunks complete', () => {
		const { updates, report } = recorder();
		for (let done = 1; done <= 8; done += 1) emitSf2WriteStep(report, done, 8, `chunk ${done}`);
		const steps = updates.map((u) => u.current);
		expect(steps).toEqual([...steps].sort((left, right) => left - right));
		expect(new Set(steps).size).toBe(steps.length);
	});

	it('treats a zero total as one unit, so a division by zero cannot produce NaN', () => {
		const { updates, report } = recorder();
		emitSf2WriteStep(report, 0, 0, 'nothing to write');
		expect(updates[0].current).toBe(61);
		expect(Number.isNaN(updates[0].current)).toBe(false);
	});
});

describe('NO_PROGRESS', () => {
	it('accepts an update and does nothing with it', () => {
		expect(() => NO_PROGRESS({ task: 'open', current: 1, total: 10, message: 'x' })).not.toThrow();
	});
});
