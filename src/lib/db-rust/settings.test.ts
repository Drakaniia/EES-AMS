import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock invoke before importing the module under test
const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
	invoke: (...args: unknown[]) => mockInvoke(...args)
}));

// Import after mock is set up
import {
	saveBrandingLogo,
	pickBrandingLogo,
	getBrandingLogoPath,
	getDefaultLogoPath,
	deleteBrandingLogo,
	resetBranding
} from './settings';

beforeEach(() => {
	vi.clearAllMocks();
});

describe('saveBrandingLogo', () => {
	it('calls invoke with save_branding_logo and file data', async () => {
		mockInvoke.mockResolvedValue('/data/assets/branding/user-abc.png');

		const fileData = [0x89, 0x50, 0x4e, 0x47]; // PNG header bytes
		const result = await saveBrandingLogo(fileData, 'logo.png');

		expect(mockInvoke).toHaveBeenCalledWith('save_branding_logo', {
			fileData,
			filename: 'logo.png'
		});
		expect(result).toBe('/data/assets/branding/user-abc.png');
	});
});

describe('pickBrandingLogo', () => {
	it('calls invoke with pick_branding_logo', async () => {
		mockInvoke.mockResolvedValue('/data/assets/branding/user-xyz.jpg');

		const result = await pickBrandingLogo();

		expect(mockInvoke).toHaveBeenCalledWith('pick_branding_logo');
		expect(result).toBe('/data/assets/branding/user-xyz.jpg');
	});
});

describe('getBrandingLogoPath', () => {
	it('calls invoke with get_branding_logo_path', async () => {
		mockInvoke.mockResolvedValue('/data/assets/branding/user-abc.png');

		const result = await getBrandingLogoPath();

		expect(mockInvoke).toHaveBeenCalledWith('get_branding_logo_path');
		expect(result).toBe('/data/assets/branding/user-abc.png');
	});

	it('returns null when no logo is set', async () => {
		mockInvoke.mockResolvedValue(null);

		const result = await getBrandingLogoPath();

		expect(result).toBeNull();
	});
});

describe('getDefaultLogoPath', () => {
	it('calls invoke with get_default_logo_path', async () => {
		mockInvoke.mockResolvedValue('__default__');

		const result = await getDefaultLogoPath();

		expect(mockInvoke).toHaveBeenCalledWith('get_default_logo_path');
		expect(result).toBe('__default__');
	});
});

describe('deleteBrandingLogo', () => {
	it('calls invoke with delete_branding_logo and path', async () => {
		mockInvoke.mockResolvedValue(undefined);

		await deleteBrandingLogo('/data/assets/branding/user-abc.png');

		expect(mockInvoke).toHaveBeenCalledWith('delete_branding_logo', {
			path: '/data/assets/branding/user-abc.png'
		});
	});
});

describe('resetBranding', () => {
	it('calls invoke with reset_branding and returns updated settings', async () => {
		const mockSettings = {
			id: 'app',
			dayStart: '08:00',
			dayEnd: '15:00',
			lateAfter: '08:45',
			quarter: '1st Quarter',
			attendanceMode: 'manual',
			brandingLogoPath: null,
			brandingTitle: 'EES AMS'
		};
		mockInvoke.mockResolvedValue(mockSettings);

		const result = await resetBranding();

		expect(mockInvoke).toHaveBeenCalledWith('reset_branding');
		expect(result.brandingLogoPath).toBeNull();
		expect(result.brandingTitle).toBe('EES AMS');
	});
});
