<script lang="ts">
	import { settingsState, quarterState } from './settings-state.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import QuarterDialog from './quarter-dialog.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
</script>

<form
	onsubmit={(e) => settingsState.onSaveGlobal(e)}
	onfocusout={(e) => settingsState.handleGlobalSettingsFocusOut(e)}
	class="space-y-5 rounded-2xl border border-border bg-card p-6"
>
	<div class="space-y-1">
		<h3 class="text-lg font-medium">Global Settings</h3>
		<p class="text-xs text-muted-foreground">
			Controls attendance flow and defaults for new classes.
		</p>
	</div>

	<div class="space-y-4">
		<fieldset class="space-y-2">
			<legend class="label-mono">Attendance Type</legend>
			<div class="grid gap-2 rounded-xl border border-border bg-surface p-1">
				<button
					type="button"
					aria-pressed={settingsState.attendanceMode === 'manual'}
					onclick={() => (settingsState.attendanceMode = 'manual')}
					class="rounded-lg border px-3 py-3 text-left transition-colors {settingsState.attendanceMode ===
					'manual'
						? 'border-primary bg-background shadow-sm'
						: 'border-transparent text-muted-foreground hover:bg-background/70 hover:text-foreground'}"
				>
					<span class="block text-sm font-semibold">Without card reader</span>
					<span class="mt-1 block text-xs leading-5">
						Name-only manual attendance for daily use.
					</span>
				</button>
				<button
					type="button"
					aria-pressed={settingsState.attendanceMode === 'card_reader'}
					onclick={() => (settingsState.attendanceMode = 'card_reader')}
					class="rounded-lg border px-3 py-3 text-left transition-colors {settingsState.attendanceMode ===
					'card_reader'
						? 'border-primary bg-background shadow-sm'
						: 'border-transparent text-muted-foreground hover:bg-background/70 hover:text-foreground'}"
				>
					<span class="block text-sm font-semibold">With card reader</span>
					<span class="mt-1 block text-xs leading-5">
						Live session optimized for ID card taps.
					</span>
				</button>
			</div>
		</fieldset>

		<div class="space-y-2">
			<label for="defDayStart" class="label-mono">Default Day Start</label>
			<input
				id="defDayStart"
				type="time"
				bind:value={settingsState.defaultDayStart}
				class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			/>
		</div>
		<div class="space-y-2">
			<label for="defDayEnd" class="label-mono">Default Day End</label>
			<input
				id="defDayEnd"
				type="time"
				bind:value={settingsState.defaultDayEnd}
				class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			/>
		</div>
		<div class="space-y-2">
			<label for="defLateAfter" class="label-mono">Default Late After</label>
			<input
				id="defLateAfter"
				type="time"
				bind:value={settingsState.defaultLateAfter}
				class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			/>
		</div>
		<div class="space-y-2">
			<label for="defQuarter" class="label-mono">Current Quarter</label>
			<button
				type="button"
				onclick={() => (quarterState.quarterDialogOpen = true)}
				class="flex h-10 w-full items-center justify-between rounded-md border border-border bg-background px-3 text-sm transition-colors hover:bg-accent/50 focus:ring-2 focus:ring-primary focus:outline-none"
			>
				<span>{quarterState.defaultQuarter}</span>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					width="16"
					height="16"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					class="opacity-50"
				>
					<path d="m6 9 6 6 6-6" />
				</svg>
			</button>
		</div>
	</div>

	<button
		type="submit"
		disabled={settingsState.globalSettingsSaving}
		class="inline-flex w-full items-center justify-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
	>
		{#if settingsState.globalSettingsSaving}
			<Spinner />
		{/if}
		{settingsState.globalSettingsSaving ? 'Saving...' : 'Save Settings'}
	</button>
</form>

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
	description="You have unsaved changes in Global Settings."
	onClose={() => settingsState.keepEditingGlobalSettings()}
>
	<div class="space-y-5">
		<div
			class="rounded-xl border border-primary/25 bg-primary/10 p-4 text-sm leading-6 text-foreground"
		>
			<div class="label-mono text-primary">Review Changes</div>
			<p class="mt-2 text-muted-foreground">
				Save your global configuration before continuing, discard the edits to restore saved values,
				or keep editing the current form.
			</p>
		</div>

		<div class="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
			<button
				type="button"
				onclick={() => settingsState.keepEditingGlobalSettings()}
				class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
			>
				Keep Editing
			</button>
			<button
				type="button"
				onclick={() => settingsState.discardGlobalSettingsChanges()}
				class="rounded-md border border-destructive/40 px-4 py-2 text-sm font-medium text-destructive transition-colors hover:bg-destructive/10"
			>
				Discard Changes
			</button>
			<button
				type="button"
				onclick={() => settingsState.saveGlobalSettingsFromDialog()}
				disabled={settingsState.globalSettingsSaving}
				class="inline-flex items-center justify-center gap-2 rounded-pill bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if settingsState.globalSettingsSaving}
					<Spinner />
				{/if}
				{settingsState.globalSettingsSaving ? 'Saving...' : 'Save Changes'}
			</button>
		</div>
	</div>
</Dialog>
