import { settingsStore } from '$lib/stores/settings.svelte';
import {
	listClasses,
	type Settings,
	type AttendanceMode
} from '$lib/features/settings/native';
import {
	buildGlobalSettingsPayload,
	globalSettingsEqual,
	normalizeGlobalSettings
} from '$lib/features/settings/global-settings';
import { classState } from './class-state.svelte';
import { backupState } from './backup-state.svelte';
import { sf2State } from './sf2-state.svelte';
import { quarterState } from './quarter-state.svelte';
import type { Ctx } from './state-context';

/**
 * Settings page orchestrator.
 * Coordinates across sub-states and owns cross-cutting concerns:
 * global settings, toast notifications.
 */
class SettingsPageState implements Ctx {
	constructor() {
		// Wire cross-cutting services into sub-state singletons
		classState.init(this);
		backupState.init(this);
		sf2State.init(this);
	}

	// ── Sub-state references (same singletons exported from sub-state files) ───
	classState = classState;
	backupState = backupState;
	sf2State = sf2State;
	quarterState = quarterState;

	// ── Global settings ────────────────────────────────────────────────────────
	defaultDayStart = $state('08:00');
	defaultDayEnd = $state('15:00');
	defaultLateAfter = $state('08:45');
	attendanceMode = $state<AttendanceMode>('manual');

	savedGlobalSettingsSnapshot = $state<Settings | null>(null);
	pendingGlobalSettingsReload = $state<Settings | null>(null);
	unsavedGlobalDialogOpen = $state(false);
	globalSettingsSaving = $state(false);

	globalSettingsDirty = $derived.by(
		() =>
			this.savedGlobalSettingsSnapshot !== null &&
			!globalSettingsEqual(this.currentSettingsPayload(), this.savedGlobalSettingsSnapshot)
	);

	currentSettingsPayload(): Settings {
		return buildGlobalSettingsPayload({
			dayStart: this.defaultDayStart,
			dayEnd: this.defaultDayEnd,
			lateAfter: this.defaultLateAfter,
			quarter: this.quarterState.defaultQuarter,
			attendanceMode: this.attendanceMode,
			q1Start: this.quarterState.q1Start,
			q1End: this.quarterState.q1End,
			q2Start: this.quarterState.q2Start,
			q2End: this.quarterState.q2End,
			q3Start: this.quarterState.q3Start,
			q3End: this.quarterState.q3End
		});
	}

	applyGlobalSettings(settings: Settings) {
		const normalized = normalizeGlobalSettings(settings);
		this.defaultDayStart = normalized.dayStart;
		this.defaultDayEnd = normalized.dayEnd;
		this.defaultLateAfter = normalized.lateAfter;
		this.quarterState.defaultQuarter = normalized.quarter;
		this.attendanceMode = normalized.attendanceMode;
		this.quarterState.q1Start = normalized.q1Start ?? '';
		this.quarterState.q1End = normalized.q1End ?? '';
		this.quarterState.q2Start = normalized.q2Start ?? '';
		this.quarterState.q2End = normalized.q2End ?? '';
		this.quarterState.q3Start = normalized.q3Start ?? '';
		this.quarterState.q3End = normalized.q3End ?? '';
		this.savedGlobalSettingsSnapshot = normalized;
		this.pendingGlobalSettingsReload = null;
	}

	handleGlobalSettingsFocusOut(event: FocusEvent) {
		if (!this.globalSettingsDirty || this.unsavedGlobalDialogOpen) return;
		const currentTarget = event.currentTarget as HTMLElement;
		const nextTarget = event.relatedTarget;
		if (nextTarget instanceof Node && currentTarget.contains(nextTarget)) return;
		this.unsavedGlobalDialogOpen = true;
	}

	async saveGlobalSettings() {
		if (this.globalSettingsSaving) return false;
		this.globalSettingsSaving = true;
		try {
			const savedSettings = await settingsStore.save(this.currentSettingsPayload());
			this.applyGlobalSettings(savedSettings);
			this.unsavedGlobalDialogOpen = false;
			this.toast('Global configuration saved');
			return true;
		} catch (error) {
			const msg = this.errorMessage(error, 'Failed to save settings');
			this.toast(`Save failed: ${msg}`, false);
			return false;
		} finally {
			this.globalSettingsSaving = false;
		}
	}

	async onSaveGlobal(e: SubmitEvent) {
		e.preventDefault();
		await this.saveGlobalSettings();
	}

	keepEditingGlobalSettings() {
		this.pendingGlobalSettingsReload = null;
		this.unsavedGlobalDialogOpen = false;
	}

	discardGlobalSettingsChanges() {
		const settingsToApply = this.pendingGlobalSettingsReload ?? this.savedGlobalSettingsSnapshot;
		if (settingsToApply) {
			this.applyGlobalSettings(settingsToApply);
		}
		this.unsavedGlobalDialogOpen = false;
	}

	async saveGlobalSettingsFromDialog() {
		await this.saveGlobalSettings();
	}

	// ── Toast ──────────────────────────────────────────────────────────────────
	toastMessage = $state<string | null>(null);
	toastOk = $state(true);
	toastTimer: ReturnType<typeof setTimeout> | null = null;

	toast(msg: string, ok = true) {
		this.toastMessage = msg;
		this.toastOk = ok;
		if (this.toastTimer) clearTimeout(this.toastTimer);
		this.toastTimer = setTimeout(() => (this.toastMessage = null), 3000);
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────
	errorMessage(error: unknown, fallback: string): string {
		if (error instanceof Error) return error.message;
		if (typeof error === 'string') return error;
		return fallback;
	}

	// ── Lifecycle ──────────────────────────────────────────────────────────────
	init() {
		this.reload();
		this.backupState.reloadBackups();
	}

	async reload() {
		try {
			const [c] = await Promise.all([listClasses(), settingsStore.load()]);
			this.classState.classes = c;
			if (settingsStore.settings) {
				const loadedSettings = normalizeGlobalSettings(settingsStore.settings);
				if (this.globalSettingsDirty) {
					this.pendingGlobalSettingsReload = loadedSettings;
					this.unsavedGlobalDialogOpen = true;
					return;
				}
				this.applyGlobalSettings(loadedSettings);
			}
		} catch (err: unknown) {
			const msg = this.errorMessage(err, 'Database error');
			this.toast(`Failed to load: ${msg}`, false);
		}
	}

}

export const settingsState = new SettingsPageState();

// Re-export sub-state singletons for direct component imports.
// Components can `import { classState } from './settings-state.svelte'`
// or `import { classState } from './class-state.svelte'` — both point to
// the same instances created above.
export { classState, backupState, sf2State, quarterState };
