import { animate } from 'motion';
import { prefersReducedMotion } from '$lib/motion';

export type ThemeMode = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

const STORAGE_KEY = 'ees-ams-theme';

function systemTheme(): ResolvedTheme {
	if (typeof window === 'undefined' || !window.matchMedia) return 'light';
	return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

/**
 * Light/dark is a UI preference, not school data, so it lives in localStorage
 * rather than the SQLite settings row. The resolved class is applied to
 * `documentElement`, which is what the `.dark` token block keys off.
 */
class ThemeStore {
	mode = $state<ThemeMode>('system');
	resolved = $state<ResolvedTheme>('light');

	private media: MediaQueryList | null = null;
	private started = false;

	init(): void {
		if (this.started || typeof window === 'undefined') return;
		this.started = true;

		const stored = window.localStorage.getItem(STORAGE_KEY);
		this.mode = stored === 'light' || stored === 'dark' || stored === 'system' ? stored : 'system';
		this.apply(false);

		this.media = window.matchMedia('(prefers-color-scheme: dark)');
		this.media.addEventListener('change', this.onSystemChange);
	}

	destroy(): void {
		this.media?.removeEventListener('change', this.onSystemChange);
		this.started = false;
	}

	set(mode: ThemeMode): void {
		this.mode = mode;
		window.localStorage.setItem(STORAGE_KEY, mode);
		this.apply(true);
	}

	private onSystemChange = (): void => {
		if (this.mode === 'system') this.apply(true);
	};

	private apply(animated: boolean): void {
		const next = this.mode === 'system' ? systemTheme() : this.mode;
		const root = document.documentElement;
		const changed = next !== this.resolved;

		this.resolved = next;
		root.classList.toggle('dark', next === 'dark');
		// The scrollbar and form-control chrome follow the document, not the token.
		root.style.colorScheme = next;

		if (animated && changed && !prefersReducedMotion()) {
			// A theme flip is a full-screen brightness jump. Fade the transition
			// rather than cutting to the new palette in one frame.
			animate(root, { opacity: [1, 0.72, 1] }, { duration: 0.3, ease: 'easeInOut' });
		}
	}
}

export const themeStore = new ThemeStore();
