import {
	getSettings,
	saveSettings as saveSettingsApi,
	type Settings
} from '$lib/features/settings/native';

/**
 * Reactive settings store using Svelte 5 runes
 * Provides instant updates across all components when settings change
 */
class SettingsStore {
	private _settings = $state<Settings | null>(null);
	private _loading = $state(true);
	private _error = $state<string | null>(null);

	constructor() {
		this.load();
	}

	get settings() {
		return this._settings;
	}

	get loading() {
		return this._loading;
	}

	get error() {
		return this._error;
	}

	/**
	 * Load settings from the backend
	 */
	async load() {
		try {
			this._loading = true;
			this._error = null;
			this._settings = await getSettings();
		} catch (error) {
			this._error = error instanceof Error ? error.message : 'Failed to load settings';
			console.error('Failed to load settings:', error);
			// Set fallback settings to prevent UI from breaking
			this._settings = {
				id: 'app',
				dayStart: '08:30',
				dayEnd: '15:30',
				lateAfter: '08:45',
				quarter: '1st Quarter',
				attendanceMode: 'manual',
				q1Start: '',
				q1End: '',
				q2Start: '',
				q2End: '',
				q3Start: '',
				q3End: ''
			};
		} finally {
			this._loading = false;
		}
	}

	/**
	 * Save settings to the backend and update the store
	 */
	async save(newSettings: Settings) {
		try {
			this._error = null;
			const savedSettings = await saveSettingsApi(newSettings);
			// Update the reactive state immediately
			this._settings = savedSettings;
			return savedSettings;
		} catch (error) {
			this._error = error instanceof Error ? error.message : 'Failed to save settings';
			throw error;
		}
	}

	/**
	 * Update specific setting fields
	 */
	async update(updates: Partial<Omit<Settings, 'id'>>) {
		if (!this._settings) {
			throw new Error('Settings not loaded');
		}

		const updatedSettings: Settings = {
			...this._settings,
			...updates
		};

		return this.save(updatedSettings);
	}
}

// Export singleton instance
export const settingsStore = new SettingsStore();
