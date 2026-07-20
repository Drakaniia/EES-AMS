import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [sveltekit()],
	test: {
		include: ['src/**/*.{test,spec}.{ts,js}'],
		environment: 'jsdom',
		setupFiles: ['./src/test-setup.ts'],
		globals: true,
		testTimeout: 60000,
		pool: 'forks'
	},
	resolve: {
		conditions: ['browser', 'development']
	}
});
