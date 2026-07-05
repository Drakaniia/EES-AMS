<script lang="ts">
	import { settingsState } from './settings-state.svelte.ts';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { ShieldCheck } from 'lucide-svelte';
	import { formatBackupBytes } from '$lib/features/settings/backup';
</script>

<Dialog
	open={settingsState.restorePreview !== null}
	title="Restore Backup"
	description="Review this SQLite backup before replacing the current database."
	maxWidth="lg"
	onClose={() => {
		if (!settingsState.restoreBusy) settingsState.restorePreview = null;
	}}
>
	{#if settingsState.restorePreview}
		<div class="space-y-4">
			<div class="rounded-xl border border-border bg-surface p-4">
				<div class="flex items-start gap-3">
					<div
						class="mt-0.5 flex size-9 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary"
					>
						<ShieldCheck class="size-5" aria-hidden="true" />
					</div>
					<div class="min-w-0">
						<div class="text-sm font-semibold">{settingsState.restorePreview.fileName}</div>
						<div class="mt-1 text-xs break-all text-muted-foreground">
							{settingsState.restorePreview.sourcePath}
						</div>
					</div>
				</div>
			</div>

			<div class="grid gap-3 sm:grid-cols-3">
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Students</div>
					<div class="mt-2 text-2xl font-semibold">{settingsState.restorePreview.studentCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Classes</div>
					<div class="mt-2 text-2xl font-semibold">{settingsState.restorePreview.classCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Attendance</div>
					<div class="mt-2 text-2xl font-semibold">{settingsState.restorePreview.eventCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Settings</div>
					<div class="mt-2 text-2xl font-semibold">{settingsState.restorePreview.settingsCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">SF2 Templates</div>
					<div class="mt-2 text-2xl font-semibold">{settingsState.restorePreview.sf2TemplateCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Size</div>
					<div class="mt-2 text-2xl font-semibold">
						{formatBackupBytes(settingsState.restorePreview.sizeBytes)}
					</div>
				</div>
			</div>

			<div class="rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
				Restoring replaces the current database. A pre-restore safety backup is created first, and
				restore stops if that safety backup cannot be created.
			</div>

			{#if settingsState.restorePreview.warnings.length > 0}
				<div class="rounded-xl border border-border p-4 text-sm text-muted-foreground">
					{#each settingsState.restorePreview.warnings as warning (warning)}
						<div>{warning}</div>
					{/each}
				</div>
			{/if}

			<div class="flex justify-end gap-2 pt-2">
				<button
					type="button"
					onclick={() => (settingsState.restorePreview = null)}
					disabled={settingsState.restoreBusy}
					class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={() => settingsState.onConfirmRestoreBackup()}
					disabled={settingsState.restoreBusy}
					class="inline-flex items-center justify-center gap-2 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white transition-colors hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if settingsState.restoreBusy}
						<Spinner />
					{/if}
					{settingsState.restoreBusy ? 'Restoring...' : 'Restore Database'}
				</button>
			</div>
		</div>
	{/if}
</Dialog>
