import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import BrandingSection from './branding-section.svelte';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
	convertFileSrc: (path: string) => `asset://localhost/${path}`
}));

// Mock settings store
const mockSettings = {
	settings: {
		quarter: '1st Quarter',
		attendanceMode: 'manual',
		brandingLogoPath: null as string | null,
		brandingTitle: 'EES AMS'
	},
	loading: false,
	error: null,
	load: vi.fn().mockResolvedValue(undefined)
};

vi.mock('$lib/stores/settings.svelte', () => ({
	get settingsStore() {
		return mockSettings;
	}
}));

// Mock branding commands
const mockPickBrandingLogo = vi.fn();
const mockResetBranding = vi.fn();

vi.mock('$lib/features/settings/native', () => ({
	get pickBrandingLogo() {
		return mockPickBrandingLogo;
	},
	get resetBranding() {
		return mockResetBranding;
	},
	saveBrandingLogo: vi.fn(),
	getBrandingLogoPath: vi.fn(),
	getDefaultLogoPath: vi.fn(),
	deleteBrandingLogo: vi.fn()
}));

// Mock settings state
const mockToast = vi.fn();
vi.mock('./settings-state.svelte', () => ({
	get settingsState() {
		return {
			brandingLogoPath: null,
			brandingTitle: 'EES AMS',
			toast: mockToast
		};
	}
}));

beforeEach(() => {
	vi.clearAllMocks();
	mockSettings.settings = {
		quarter: '1st Quarter',
		attendanceMode: 'manual',
		brandingLogoPath: null,
		brandingTitle: 'EES AMS'
	};
	mockSettings.load.mockResolvedValue(undefined);
	mockResetBranding.mockResolvedValue({
		id: 'app',
		brandingLogoPath: null,
		brandingTitle: 'EES AMS'
	});
});

describe('BrandingSection', () => {
	it('renders the app branding heading', () => {
		render(BrandingSection);
		expect(screen.getByText('App Branding')).toBeInTheDocument();
	});

	it('displays the current title in the input field', () => {
		render(BrandingSection);
		const input = screen.getByLabelText('Title Text');
		expect(input).toHaveValue('EES AMS');
	});

	it('shows character count for the title', () => {
		render(BrandingSection);
		expect(screen.getByText('7/40')).toBeInTheDocument();
	});

	it('updates character count as user types', async () => {
		render(BrandingSection);
		const input = screen.getByLabelText('Title Text');

		await fireEvent.input(input, { target: { value: 'Mrs. Santos' } });
		expect(screen.getByText('11/40')).toBeInTheDocument();
	});

	it('renders upload button', () => {
		render(BrandingSection);
		expect(screen.getByText('Upload Image')).toBeInTheDocument();
	});

	it('renders reset to default button', () => {
		render(BrandingSection);
		expect(screen.getByText('Reset to Default')).toBeInTheDocument();
	});

	it('shows preset gallery with 4 presets', () => {
		render(BrandingSection);
		expect(screen.getByText('School Seal')).toBeInTheDocument();
		expect(screen.getByText('Graduation')).toBeInTheDocument();
		expect(screen.getByText('Book')).toBeInTheDocument();
		expect(screen.getByText('Apple')).toBeInTheDocument();
	});

	it('displays the tip text', () => {
		render(BrandingSection);
		expect(screen.getByText(/Enter a title like/)).toBeInTheDocument();
	});

	it('displays custom title from settings', () => {
		mockSettings.settings.brandingTitle = 'Room 201 - Grade 3';
		render(BrandingSection);
		const input = screen.getByLabelText('Title Text');
		expect(input).toHaveValue('Room 201 - Grade 3');
	});
});
