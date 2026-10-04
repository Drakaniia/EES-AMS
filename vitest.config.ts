import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [sveltekit()],
	test: {
		include: ['src/**/*.{test,spec}.{ts,js}'],
		// A wall-clock budget cannot be measured while 66 other files compete for
		// the CPU — p50 tracks machine load roughly 1:1. So this file is excluded
		// by default and `bun run test:perf` sets VITEST_PERF to run it alone;
		// `bun run test` stays deterministic.
		exclude:
			process.env.VITEST_PERF === '1'
				? ['**/node_modules/**']
				: ['**/node_modules/**', '**/month-switch-perf.test.ts'],
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
