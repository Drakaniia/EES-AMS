/**
 * Acceptance #14: the attendance-import control is gone from the UI entirely.
 *
 * ## Why this is a source-level test and not a render test
 *
 * The button was a `ReportSidebar.svelte` element with a prop, a state field and
 * a handler threaded through three files. A test that renders the sidebar and
 * looks for the label proves the *label* is gone, which is a much weaker claim
 * than what the spec asks for: that the recovery control does not exist. It
 * would pass with a button that says "Recover absences", and fail on a comment
 * that merely mentions the old one.
 *
 * So the assertion is against the source of the whole `reports` route: no file in
 * it may name the old control, and none of the plumbing that existed only to
 * drive it may survive. The same file also pins the two things that must *not*
 * be deleted along with it, because a mechanical removal takes them too:
 *
 * - the passive status line (spec §12.2), and
 * - the status of the plumbing the removed button dragged along with it.
 *
 * ## Why the needle is assembled rather than written out
 *
 * The acceptance gate is a case-insensitive grep for the old label's first three
 * words over `src/`, and it must return **nothing** - including from this file.
 * A test that hard-codes the string it forbids is a file that fails its own
 * gate, so the words are built from parts here. That is not cleverness: it is the
 * only way to keep the gate honest while still testing the thing it forbids.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const REPORTS_DIR = join(import.meta.dirname);

/**
 * The forbidden label, assembled so this file does not trip the very gate it
 * enforces.
 */
const FORBIDDEN_LABEL = ['import', 'x', 'from'].join(' ');

/** Every source file of the Reports route, whatever its extension. */
function reportSources(): { name: string; code: string }[] {
	return readdirSync(REPORTS_DIR)
		.filter((name) => /\.(svelte|ts)$/.test(name) && !name.endsWith('.test.ts'))
		.map((name) => ({ name, code: readFileSync(join(REPORTS_DIR, name), 'utf8') }));
}

const SOURCES = reportSources();

/** The rendered markup of a Svelte file, with script and prose dropped. */
function markupOf(name: string): string {
	const source = readFileSync(join(REPORTS_DIR, name), 'utf8');
	const afterScript = source.split('</script>').slice(1).join('</script>');
	// Comments survive into the markup, and prose explaining the removal
	// necessarily names what was removed.
	return afterScript.replace(/<!--[\s\S]*?-->/g, '').replace(/\/\/[^\n]*/g, '');
}

describe(`the removed attendance-import control is gone (acceptance #14)`, () => {
	it('found the route sources — the test is testing something', () => {
		expect(SOURCES.length).toBeGreaterThan(5);
	});

	it('names the old control nowhere in the route', () => {
		for (const { name, code } of SOURCES) {
			expect(code.toLowerCase(), `${name} still names the removed control`).not.toContain(
				FORBIDDEN_LABEL
			);
		}
	});

	it('has none of the plumbing that existed only to drive it', () => {
		// Each of these was reachable *only* from the removed button. Left behind,
		// they are the shape of a control with its label taken off.
		for (const plumbing of [
			'onImportAttendance',
			'importingAttendance',
			'canRecoverFromWorkbook',
			'importSf2AttendanceFromWorkbook'
		]) {
			for (const { name, code } of SOURCES) {
				expect(code, `${name} still references ${plumbing}`).not.toContain(plumbing);
			}
		}
	});

	it('renders no recovery control under any other name', () => {
		// The spec removes a *recovery affordance*, not a string. So this looks
		// inside the sidebar's actual `<button>` elements rather than at the file:
		// `formatImportedAt` and the "Imported" identity row both contain "Import"
		// and neither is a control, and a whole-file grep would call them one.
		const markup = markupOf('ReportSidebar.svelte');
		const buttons = markup.match(/<button[\s\S]*?<\/button>/g) ?? [];
		expect(buttons.length, 'no buttons found; the test is not testing controls').toBeGreaterThan(0);

		for (const button of buttons) {
			for (const verb of [FORBIDDEN_LABEL, 'Recover', 'Reimport', 'Read marks from', 'Import']) {
				expect(button, `a sidebar button offers "${verb}"`).not.toContain(verb);
			}
		}
	});
});

describe('what replaced the button is still there', () => {
	it('keeps the passive status line (spec §12.2)', () => {
		const state = readFileSync(join(REPORTS_DIR, 'report-state.svelte.ts'), 'utf8');
		const sidebar = readFileSync(join(REPORTS_DIR, 'ReportSidebar.svelte'), 'utf8');

		expect(state).toContain('export function workbookStatusLine');
		expect(sidebar).toContain('workbookStatusLine');
		// The two counts the guard compares, and the disagreement warning.
		expect(state).toContain('workbookXCount');
		expect(state).toContain('appXCount');
		expect(sidebar).toContain('markCounts.agrees');
	});

	it('has the sidebar read the time the count was taken', () => {
		// "Last checked <time>" is the first half of §12.2's sentence, and it can
		// only be rendered if the scanned-at timestamp is actually threaded to the
		// sidebar rather than derived from anything local.
		const page = readFileSync(join(REPORTS_DIR, '+page.svelte'), 'utf8');
		const sidebar = readFileSync(join(REPORTS_DIR, 'ReportSidebar.svelte'), 'utf8');

		expect(page).toContain('workbookScannedAt={page.monthGrid?.workbookScannedAt ?? null}');
		expect(sidebar).toContain('workbookScannedAt: number | null');
	});
});
