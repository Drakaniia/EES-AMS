/**
 * The launch toast decision, across all seven outcomes of §8.2.
 *
 * ## Why this is a unit test and not a render test
 *
 * Acceptance #15 is "launching the app imports them **and toasts the count**". The
 * toast half is one branch in one pure function; the rest of it is a subscriber
 * and a `<div>`. Rendering the toast to assert it appears would prove that
 * `FeedbackToast` renders strings, in an environment where 22 sibling tests
 * already fail on `document is not defined` - it would trade a real assertion for
 * a new member of that list.
 *
 * So the assertion is on the decision, and it is deliberately exhaustive: every
 * one of the seven `Sf2HealOutcome` variants is named here. A sixth or seventh
 * variant added in Rust shows up as a compile error at
 * {@link ALL_STATUSES_IS_EXHAUSTIVE}, and a variant that starts toasting shows up
 * as a failure in {@link the four boring outcomes are silent}.
 */
import { describe, it, expect } from 'vitest';
import {
	sf2HealNotice,
	sf2HealQuietNotice,
	sf2HealToast,
	SF2_HEAL_SILENT_STATUSES,
	type Sf2HealNotice
} from './sf2-heal-toast';
import type { Sf2HealOutcome } from '$lib/types';

const RECOVERED: Sf2HealOutcome = {
	status: 'recovered',
	month: 'SEPTEMBER',
	fileName: 'SF2-SEPTEMBER-2026.xls',
	imported: 12,
	alreadyRecorded: 3,
	workbookCount: 15,
	dbCountBefore: 3,
	dbCountAfter: 15,
	scannedAt: 1_789_000_000,
	toast: 'Recovered 12 X marks from SF2-SEPTEMBER-2026.xls'
};

const UP_TO_DATE: Sf2HealOutcome = {
	status: 'upToDate',
	month: 'SEPTEMBER',
	dbCount: 15,
	workbookCount: 12,
	scannedAt: 1_789_000_000
};

const IN_SYNC: Sf2HealOutcome = {
	status: 'inSync',
	month: 'SEPTEMBER',
	dbCount: 12,
	workbookCount: 12,
	scannedAt: 1_789_000_000
};

const ALL_OUTCOMES: Record<Sf2HealOutcome['status'], Sf2HealOutcome> = {
	alreadyRan: { status: 'alreadyRan' },
	notApplicable: { status: 'notApplicable', reason: 'No SF2 workbook is stored for JUNE.' },
	excelUnavailable: { status: 'excelUnavailable', reason: 'The workbook could not be read.' },
	workbookMissing: {
		status: 'workbookMissing',
		reason: 'SF2-SEPTEMBER-2026.xls is missing. Restore it from a backup.'
	},
	upToDate: UP_TO_DATE,
	inSync: IN_SYNC,
	recovered: RECOVERED
};

/**
 * The wire names, spelled out.
 *
 * `serde(tag = "status", rename_all = "camelCase")` is what decides these, and a
 * mismatch here is silent: the frontend's discriminant would never match a real
 * payload, `sf2HealNotice` would return `null` for a genuine recovery, and
 * acceptance #15 would fail with nothing in the log. So the mapping is asserted
 * rather than assumed.
 */
describe('the outcome wire format', () => {
	it('uses the seven camelCase variant names Rust serialises', () => {
		expect(Object.keys(ALL_OUTCOMES).sort()).toEqual([
			'alreadyRan',
			'excelUnavailable',
			'inSync',
			'notApplicable',
			'recovered',
			'upToDate',
			'workbookMissing'
		]);
	});

	it('carries the camelCase field names the Recovered variant needs', () => {
		// The two fields the toast is built from. A `snake_case` slip on either
		// would make the count read `undefined`.
		expect(RECOVERED.fileName).toBe('SF2-SEPTEMBER-2026.xls');
		expect(RECOVERED.imported).toBe(12);
		expect(RECOVERED.toast).toContain('12');
	});
});

describe('the launch toast decision', () => {
	it("toasts the recovered count, in the backend's own words", () => {
		// Acceptance #15, precisely: the count, and the file it came from.
		expect(sf2HealToast(RECOVERED)).toBe('Recovered 12 X marks from SF2-SEPTEMBER-2026.xls');
	});

	it('falls back to a composed sentence when the toast string arrives empty', () => {
		const blank: Sf2HealOutcome = { ...RECOVERED, toast: '   ' };
		expect(sf2HealToast(blank)).toBe('Recovered 12 X marks from SF2-SEPTEMBER-2026.xls');
	});

	it('toasts a recovery of zero rather than swallowing it', () => {
		// `imported: 0` with a `Recovered` outcome is a real (if odd) report: the
		// counts said the file was ahead and the cells turned out already recorded.
		// Hiding it would make the guard's own comparison unexplainable.
		const zero: Sf2HealOutcome = { ...RECOVERED, imported: 0, toast: 'Recovered 0 X marks' };
		expect(sf2HealToast(zero)).toBe('Recovered 0 X marks');
	});

	it('names exactly four silent outcomes, and they are the boring ones', () => {
		expect([...SF2_HEAL_SILENT_STATUSES].sort()).toEqual([
			'alreadyRan',
			'inSync',
			'notApplicable',
			'upToDate'
		]);
	});

	it.each(['alreadyRan', 'notApplicable', 'upToDate', 'inSync'] as const)(
		'%s produces no toast and no notice at all',
		(status) => {
			// A toast on every launch is the failure mode. `null` from
			// `sf2HealNotice` - not a short message, not an empty one - is what
			// keeps the real recovery readable.
			expect(sf2HealToast(ALL_OUTCOMES[status])).toBeNull();
			expect(sf2HealQuietNotice(ALL_OUTCOMES[status])).toBeNull();
			expect(sf2HealNotice(ALL_OUTCOMES[status])).toBeNull();
		}
	);

	it('explains an unreadable workbook quietly, and never as a failure', () => {
		const notice = sf2HealNotice(ALL_OUTCOMES.excelUnavailable);
		expect(notice).toEqual({
			kind: 'quiet',
			ok: true,
			message: 'Could not check the SF2 workbook: The workbook could not be read.'
		});
		// `ok: true` is load-bearing: the toast component renders `ok: false` as an
		// assertive, destructive-coloured alert. "Excel was not available" is not
		// an error the teacher did anything wrong about.
		expect(notice?.ok).toBe(true);
		expect(sf2HealToast(ALL_OUTCOMES.excelUnavailable)).toBeNull();
	});

	it("passes a missing workbook's own message through unchanged", () => {
		// The Rust side already words this as an instruction ("Restore it from a
		// backup"). Rewording it here would be the frontend second-guessing the
		// layer that knows whether a backup is actually the fix.
		expect(sf2HealNotice(ALL_OUTCOMES.workbookMissing)).toEqual({
			kind: 'quiet',
			ok: true,
			message: 'SF2-SEPTEMBER-2026.xls is missing. Restore it from a backup.'
		});
		expect(sf2HealToast(ALL_OUTCOMES.workbookMissing)).toBeNull();
	});

	it('decides all seven variants, with a toast for exactly one', () => {
		const decided = Object.entries(ALL_OUTCOMES).map(([status, outcome]) => [
			status,
			sf2HealNotice(outcome)?.kind ?? 'silent'
		]);
		expect(Object.fromEntries(decided)).toEqual({
			alreadyRan: 'silent',
			notApplicable: 'silent',
			excelUnavailable: 'quiet',
			workbookMissing: 'quiet',
			upToDate: 'silent',
			inSync: 'silent',
			recovered: 'toast'
		});
	});

	it('never reports a notice as both a toast and a quiet note', () => {
		for (const outcome of Object.values(ALL_OUTCOMES)) {
			const notice = sf2HealNotice(outcome) as Sf2HealNotice | null;
			const kinds = [notice?.kind].filter(Boolean);
			expect(new Set(kinds).size, `${outcome.status} produced two notices`).toBeLessThanOrEqual(1);
		}
	});

	it('keeps a reason that is an empty string visible rather than trailing a colon', () => {
		// A blank `reason` should not render as "Could not check the SF2 workbook: ".
		const notice = sf2HealNotice({ status: 'excelUnavailable', reason: '' });
		expect(notice?.message).toBe('Could not check the SF2 workbook.');
	});
});
