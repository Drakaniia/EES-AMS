<script lang="ts">
	import { settingsState, quarterState } from './settings-state.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import QuarterDialog from './quarter-dialog.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import SettingsSection from './settings-section.svelte';
	import SettingsGroup from './settings-group.svelte';
	import SettingRow from './setting-row.svelte';
	import { ChevronDown } from 'lucide-svelte';
	import { themeStore, type ThemeMode } from '$lib/stores/theme.svelte';

	const themeOptions: { value: ThemeMode; label: string }[] = [
		{ value: 'light', label: 'Light' },
		{ value: 'dark', label: 'Dark' },
		{ value: 'system', label: 'System' }
	];

	const fieldClass = 'field w-auto tabular-nums';
</script>

<SettingsSection title="General" description="Appearance and the defaults for a normal school day.">
	<SettingsGroup>
		<div class="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
			<div class="min-w-0">
				<p class="text-[13px] leading-5 font-medium">Appearance</p>
				<p class="mt-0.5 text-xs leading-4 text-muted-foreground">
					System follows your Windows theme.
				</p>
			</div>
			<div class="segmented" role="group" aria-label="Appearance">
				{#each themeOptions as option (option.value)}
					<button
						type="button"
						aria-pressed={themeStore.mode === option.value}
						onclick={() => themeStore.set(option.value)}
						class="segmented-item"
					>
						{option.label}
					</button>
				{/each}
			</div>
		</div>
	</SettingsGroup>

	<form
		onsubmit={(e) => settingsState.onSaveGlobal(e)}
		onfocusout={(e) => settingsState.handleGlobalSettingsFocusOut(e)}
		class="space-y-4"
	>
		<SettingsGroup>
			<SettingRow label="Day starts" description="When attendance opens.">
				<input
					type="time"
					aria-label="Default day start"
					bind:value={settingsState.defaultDayStart}
					class={fieldClass}
				/>
			</SettingRow>

			<SettingRow label="Day ends" description="When the school day closes.">
				<input
					type="time"
					aria-label="Default day end"
					bind:value={settingsState.defaultDayEnd}
					class={fieldClass}
				/>
			</SettingRow>

			<SettingRow label="Late after" description="Arrivals past this time are late.">
				<input
					type="time"
					aria-label="Default late after"
					bind:value={settingsState.defaultLateAfter}
					class={fieldClass}
				/>
			</SettingRow>

			<SettingRow label="Quarter" description="The grading period reports are filed under.">
				<button
					type="button"
					onclick={() => (quarterState.quarterDialogOpen = true)}
					class="field flex w-auto min-w-32 items-center justify-between gap-2"
				>
					<span>{quarterState.defaultQuarter}</span>
					<ChevronDown class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
				</button>
			</SettingRow>
		</SettingsGroup>

		<div class="flex justify-end">
			<button type="submit" disabled={settingsState.globalSettingsSaving} class="btn btn-primary">
				{#if settingsState.globalSettingsSaving}
					<Spinner />
				{/if}
				{settingsState.globalSettingsSaving ? 'Saving…' : 'Save'}
			</button>
		</div>
	</form>
</SettingsSection>

<QuarterDialog
	bind:open={quarterState.quarterDialogOpen}
	bind:quarter={quarterState.defaultQuarter}
	bind:q1Start={quarterState.q1Start}
	bind:q1End={quarterState.q1End}
	bind:q2Start={quarterState.q2Start}
	bind:q2End={quarterState.q2End}
	bind:q3Start={quarterState.q3Start}
	bind:q3End={quarterState.q3End}
/>

<!-- ── Unsaved Global Settings Dialog ── -->
<Dialog
	open={settingsState.unsavedGlobalDialogOpen}
	title="Unsaved Global Settings"
	description="You have unsaved changes in General."
	onClose={() => settingsState.keepEditingGlobalSettings()}
>
	<div class="space-y-5">
		<p class="text-sm leading-6 text-muted-foreground">
			Save your changes before continuing, discard them to restore the saved values, or keep
			editing.
		</p>

		<div class="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
			<button
				type="button"
				onclick={() => settingsState.keepEditingGlobalSettings()}
				class="btn btn-secondary"
			>
				Keep Editing
			</button>
			<button
				type="button"
				onclick={() => settingsState.discardGlobalSettingsChanges()}
				class="btn btn-danger"
			>
				Discard Changes
			</button>
			<button
				type="button"
				onclick={() => settingsState.saveGlobalSettingsFromDialog()}
				disabled={settingsState.globalSettingsSaving}
				class="btn btn-primary"
			>
				{#if settingsState.globalSettingsSaving}
					<Spinner />
				{/if}
				{settingsState.globalSettingsSaving ? 'Saving…' : 'Save Changes'}
			</button>
		</div>
	</div>
</Dialog>
