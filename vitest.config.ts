import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

// Files that dominate the suite's wall clock. Every one of them loads the real
// bundled template and drives ExcelJS through full workbook build/merge cycles,
// so a single file costs tens of seconds of CPU. Measured on a clean tree: 68
// files, 648s of summed file time, and these 16 hold ~95% of it — the other 52
// are each under 2.1s.
//
// They are not part of the default run, so `bun run test` stays the tight inner
// loop. `bun run test:heavy` (VITEST_HEAVY=1) runs everything, and that is the
// command for a pre-PR check. Nothing here is deleted or weakened: the workbook
// layout, formula-value and round-trip contracts stay fully asserted, and
// naming one of these files on the command line still runs it (see below).
const HEAVY_TESTS = [
	'src/lib/features/excel/__tests__/roster.test.ts',
	'src/lib/features/excel/__tests__/round-trip.test.ts',
	'src/lib/features/sf2/__tests__/attendance-service.test.ts',
	'src/lib/features/sf2/__tests__/attendance-write.test.ts',
	'src/lib/features/sf2/__tests__/calendar.test.ts',
	'src/lib/features/sf2/__tests__/import.test.ts',
	'src/lib/features/sf2/__tests__/roster.test.ts',
	'src/lib/features/sf2/__tests__/sf2-open-parity.test.ts',
	'src/lib/features/sf2/__tests__/template.test.ts',
	'src/lib/features/sf2/month/__tests__/merge.test.ts',
	'src/lib/features/sf2/month/__tests__/probe-twelve.test.ts',
	'src/lib/features/sf2/month/__tests__/roster-sync.test.ts',
	'src/lib/features/sf2/month/__tests__/summary-block.test.ts',
	'src/lib/features/sf2/month/__tests__/workbook-builder.test.ts',
	'src/lib/features/sf2/template/__tests__/create.test.ts',
	'src/lib/stores/command-palette.test.ts'
].map((p) => `**/${p}`);

const perfOnly = process.env.VITEST_PERF === '1';
const runHeavy = process.env.VITEST_HEAVY === '1';

// `exclude` filters even a file named on the command line, so honour an explicit
// path filter: `bunx vitest run src/lib/features/sf2/month/__tests__/merge.test.ts`
// has to work, or the heavy files would be unreachable one at a time.
const SUBCOMMANDS = new Set(['run', 'watch', 'dev', 'list', 'bench', 'related', 'init']);
const namesExplicitPath = process.argv
	.slice(2)
	.some((arg) => !arg.startsWith('-') && !SUBCOMMANDS.has(arg));

// A wall-clock budget cannot be measured while other files compete for the CPU —
// p50 tracks machine load roughly 1:1. So month-switch-perf.test.ts is excluded
// by default and `bun run test:perf` sets VITEST_PERF to run it alone.
const exclude = ['**/node_modules/**'];
if (!perfOnly) exclude.push('**/month-switch-perf.test.ts');
if (!perfOnly && !runHeavy && !namesExplicitPath) exclude.push(...HEAVY_TESTS);

export default defineConfig({
	plugins: [sveltekit()],
	test: {
		include: ['src/**/*.{test,spec}.{ts,js}'],
		exclude,
		environment: 'jsdom',
		setupFiles: ['./src/test-setup.ts'],
		globals: true,
		testTimeout: 60000
	},
	resolve: {
		conditions: ['browser', 'development'],
		external: ['node:sqlite']
	}
});
