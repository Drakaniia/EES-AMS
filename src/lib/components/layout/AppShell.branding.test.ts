import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import AppShell from './AppShell.svelte';

const mockSettings = {
	settings: {
		quarter: '1st Quarter',
		attendanceMode: 'manual',
		brandingLogoPath: null as string | null,
		brandingTitle: 'EES AMS'
	},
	loading: false,
	error: null
};

vi.mock('$app/state', () => ({
	page: { url: { pathname: '/' } }
}));

vi.mock('$app/paths', () => ({ base: '' }));
vi.mock('@tauri-apps/api/core', () => ({
	convertFileSrc: (path: string) => `asset://localhost/${path}`
}));

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

vi.mock('$lib/db-rust', () => ({ listStudents: vi.fn(async () => []) }));

vi.mock('$lib/stores/update.svelte', () => ({
	updateStore: { badgeVisible: false }
}));

vi.mock('$lib/stores/full-preview.svelte', () => ({
	fullPreviewStore: { isActive: false, isTitleBarHidden: false }
}));

vi.mock('$lib/stores/settings.svelte', () => ({
	get settingsStore() {
		return mockSettings;
	}
}));

beforeEach(() => {
	mockSettings.settings = {
		quarter: '1st Quarter',
		attendanceMode: 'manual',
		brandingLogoPath: null,
		brandingTitle: 'EES AMS'
	};
});

function renderShell() {
	return render(AppShell, {
		props: { children: () => '<div>page content</div>' }
	});
}

describe('AppShell branding in TitleBar', () => {
	it('displays default title "EES AMS" in the logo', () => {
		renderShell();
		const logo = screen.getByTitle('EES AMS');
		expect(logo).toBeInTheDocument();
	});

	it('displays custom branding title from settings', () => {
		mockSettings.settings.brandingTitle = 'Mrs. Santos - Room 201';
		renderShell();
		const logo = screen.getByTitle('Mrs. Santos - Room 201');
		expect(logo).toBeInTheDocument();
	});

	it('falls back to "EES AMS" when brandingTitle is empty', () => {
		mockSettings.settings.brandingTitle = '';
		renderShell();
		const logo = screen.getByTitle('EES AMS');
		expect(logo).toBeInTheDocument();
	});

	it('uses default logo when brandingLogoPath is null', () => {
		const { container } = renderShell();
		const logoImg = container.querySelector('.title-logo-img');
		expect(logoImg).toBeInTheDocument();
		expect(logoImg!.getAttribute('src')).not.toContain('asset://localhost');
	});

	it('uses convertFileSrc for custom logo path', () => {
		mockSettings.settings.brandingLogoPath = '/data/assets/branding/user-abc.png';
		const { container } = renderShell();
		const logoImg = container.querySelector('.title-logo-img');
		expect(logoImg).toBeInTheDocument();
		expect(logoImg!.getAttribute('src')).toBe(
			'asset://localhost//data/assets/branding/user-abc.png'
		);
	});
});
