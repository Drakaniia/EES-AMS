import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import AppShell from './AppShell.svelte';

vi.mock('$app/state', () => ({
	page: { url: { pathname: '/' } }
}));

vi.mock('$app/paths', () => ({ base: '' }));

vi.mock('$lib/stores/settings.svelte', () => ({
	settingsStore: {
		settings: {
			quarter: '1st Quarter',
			attendanceMode: 'card_reader',
			brandingLogoPath: null,
			brandingTitle: 'EES AMS'
		},
		loading: false,
		error: null
	}
}));

vi.mock('$lib/stores/update.svelte', () => ({
	updateStore: { badgeVisible: false }
}));

vi.mock('$lib/stores/full-preview.svelte', () => {
	let isActive = false;
	return {
		fullPreviewStore: {
			get isActive() {
				return isActive;
			},
			set isActive(v: boolean) {
				isActive = v;
			},
			get isTitleBarHidden() {
				return isActive;
			}
		}
	};
});
vi.mock('@tauri-apps/api/core', () => ({
	convertFileSrc: (path: string) => `asset://localhost/${path}`
}));

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

vi.mock('$lib/db-rust', () => ({ listStudents: vi.fn(async () => []) }));

function renderAppShell() {
	return render(AppShell, {
		props: {
			children: () => '<div>page content</div>'
		}
	});
}

describe('AppShell', () => {
	it('renders the TitleBar', () => {
		renderAppShell();
		const titleBar = document.querySelector('.title-bar');
		expect(titleBar).toBeInTheDocument();
	});

	it('does not render a sidebar', () => {
		renderAppShell();
		const sidebar = document.querySelector('aside');
		expect(sidebar).not.toBeInTheDocument();
	});

	it('renders the main content area', () => {
		renderAppShell();
		const main = screen.getByRole('main');
		expect(main).toBeInTheDocument();
	});

	it('renders navigation links in the TitleBar', () => {
		renderAppShell();
		const navLinks = document.querySelectorAll('.title-nav-link');
		expect(navLinks.length).toBe(2);
	});

	it('hides TitleBar in full-preview mode', async () => {
		const { fullPreviewStore } = await import('$lib/stores/full-preview.svelte');
		fullPreviewStore.isActive = true;

		renderAppShell();
		const titleBar = document.querySelector('.title-bar');
		expect(titleBar).not.toBeInTheDocument();

		// Reset
		fullPreviewStore.isActive = false;
	});
});
