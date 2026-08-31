import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import AppShell from './AppShell.svelte';

// Mutable settings mock — tests can modify mockSettings.settings between renders
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

vi.mock('$lib/stores/update.svelte', () => ({
	updateStore: { badgeVisible: false }
}));

vi.mock('$lib/stores/full-preview.svelte', () => ({
	fullPreviewStore: { isActive: false }
}));

vi.mock('$lib/stores/settings.svelte', () => ({
	get settingsStore() {
		return mockSettings;
	}
}));

beforeEach(() => {
	// Reset to defaults before each test
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

describe('AppShell sidebar branding', () => {
	it('displays default title "EES AMS" when no custom title is set', () => {
		renderShell();
		const title = screen.getByText('EES AMS');
		expect(title).toBeInTheDocument();
	});

	it('displays custom branding title from settings', () => {
		mockSettings.settings.brandingTitle = 'Mrs. Santos - Room 201';
		renderShell();
		const title = screen.getByText('Mrs. Santos - Room 201');
		expect(title).toBeInTheDocument();
	});

	it('falls back to "EES AMS" when brandingTitle is empty string', () => {
		mockSettings.settings.brandingTitle = '';
		renderShell();
		const title = screen.getByText('EES AMS');
		expect(title).toBeInTheDocument();
	});

	it('uses default logo when brandingLogoPath is null', () => {
		const { container } = renderShell();
		const logoImg = container.querySelector('img[alt]');
		expect(logoImg).toBeInTheDocument();
		expect(logoImg!.getAttribute('src')).not.toContain('asset://localhost');
	});

	it('uses convertFileSrc for custom logo path', () => {
		mockSettings.settings.brandingLogoPath = '/data/assets/branding/user-abc.png';
		const { container } = renderShell();
		const logoImg = container.querySelector('img[alt]');
		expect(logoImg).toBeInTheDocument();
		expect(logoImg!.getAttribute('src')).toBe(
			'asset://localhost//data/assets/branding/user-abc.png'
		);
	});

	it('sets alt text to the branding title', () => {
		mockSettings.settings.brandingTitle = 'My Custom School';
		renderShell();
		const logoImg = screen.getByAltText('My Custom School');
		expect(logoImg).toBeInTheDocument();
	});

	it('removes the custom title from DOM when sidebar is collapsed', async () => {
		mockSettings.settings.brandingTitle = 'Custom Title';
		renderShell();
		const toggleBtn = screen.getByLabelText('Toggle sidebar');

		// Title should be present before collapse
		expect(screen.getByText('Custom Title')).toBeInTheDocument();

		// Collapse sidebar
		await fireEvent.click(toggleBtn);

		// Title should no longer be in the DOM (conditionally rendered)
		expect(screen.queryByText('Custom Title')).not.toBeInTheDocument();
	});
});
