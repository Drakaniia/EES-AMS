<script lang="ts">
	import { backupState } from './settings-state.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { ShieldCheck } from 'lucide-svelte';
	import { formatBackupBytes } from '$lib/features/settings/backup';
</script>

<Dialog
	open={backupState.restorePreview !== null}
	title="Restore Backup"
	description="Review this backup before replacing the current database."
	maxWidth="lg"
	onClose={() => {
		if (!backupState.restoreBusy) backupState.restorePreview = null;
	}}
>
	{#if backupState.restorePreview}
		<div class="space-y-4">
			<div class="rounded-xl border border-border bg-surface p-4">
				<div class="flex items-start gap-3">
					<div
						class="mt-0.5 flex size-9 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary"
					>
						<ShieldCheck class="size-5" aria-hidden="true" />
					</div>
					<div class="min-w-0">
						<div class="text-sm font-semibold">{backupState.restorePreview.fileName}</div>
						<div class="mt-1 text-xs break-all text-muted-foreground">
							{backupState.restorePreview.sourcePath}
						</div>
					</div>
				</div>
			</div>

			<div class="grid gap-3 sm:grid-cols-3">
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Students</div>
					<div class="mt-2 text-2xl font-semibold">{backupState.restorePreview.studentCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Classes</div>
					<div class="mt-2 text-2xl font-semibold">{backupState.restorePreview.classCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Attendance</div>
					<div class="mt-2 text-2xl font-semibold">{backupState.restorePreview.eventCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Absences</div>
					<div class="mt-2 text-2xl font-semibold">{backupState.restorePreview.absentCount}</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Settings</div>
					<div class="mt-2 text-2xl font-semibold">
						{backupState.restorePreview.settingsCount}
					</div>
				</div>
				<div class="rounded-xl border border-border p-4">
					<div class="label-mono">Size</div>
					<div class="mt-2 text-2xl font-semibold">
						{formatBackupBytes(backupState.restorePreview.sizeBytes)}
					</div>
				</div>
			</div>

			<div class="rounded-xl border border-border bg-background p-4">
				<div class="label-mono">SF2 Workbooks in This Backup</div>
				{#if backupState.restorePreview.workbooks.length === 0}
					<p class="mt-3 text-sm text-muted-foreground">
						None. This backup holds only the database, so your current SF2 workbooks are left
						untouched.
					</p>
				{:else}
					<div class="mt-3 overflow-x-auto">
						<table class="w-full text-left text-sm">
							<thead>
								<tr class="border-b border-border text-[11px] text-muted-foreground">
									<th class="py-1.5 pr-3 font-medium">File</th>
									<th class="py-1.5 pr-3 font-medium">Size</th>
									<th class="py-1.5 font-medium">X marks</th>
								</tr>
							</thead>
							<tbody>
								{#each backupState.restorePreview.workbooks as workbook (workbook.relativePath)}
									<tr class="border-b border-border/60 last:border-0">
										<td class="py-1.5 pr-3 font-mono text-xs break-all">
											{workbook.fileName}
										</td>
										<td class="py-1.5 pr-3 font-mono text-xs text-muted-foreground">
											{formatBackupBytes(workbook.bytes)}
										</td>
										<td class="py-1.5 font-mono text-xs font-semibold">{workbook.xCount}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>

			{#if backupState.workbookAheadCount}
				<div
					class="rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive"
				>
					<div class="font-semibold">These workbooks are ahead of their own database</div>
					<div class="mt-1">
						The workbooks record {backupState.workbookAheadCount.expected} X mark(s) but this backup's
						database holds only {backupState.workbookAheadCount.actual} absence(s). Restoring pairs them
						as they are; the app re-imports the missing marks the next time you open SF2. Only continue
						if that is what you want.
					</div>
				</div>
			{/if}

			<div class="rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
				Restoring replaces the current database{backupState.restorePreview.workbooks.length > 0
					? ' and overwrites the current SF2 workbooks'
					: ''}. A pre-restore safety backup — including the workbooks — is created first, and
				restore stops if that safety backup cannot be created.
			</div>

			{#if backupState.restorePreview.warnings.length > 0}
				<div class="rounded-xl border border-border p-4 text-sm text-muted-foreground">
					{#each backupState.restorePreview.warnings as warning (warning)}
						<div>{warning}</div>
					{/each}
				</div>
			{/if}

			<div class="flex justify-end gap-2 pt-2">
				<button
					type="button"
					onclick={() => (backupState.restorePreview = null)}
					disabled={backupState.restoreBusy}
					class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={() => backupState.onConfirmRestoreBackup()}
					disabled={backupState.restoreBusy}
					class="inline-flex items-center justify-center gap-2 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white transition-colors hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if backupState.restoreBusy}
						<Spinner />
					{/if}
					{backupState.restoreBusy ? 'Restoring...' : 'Restore Backup'}
				</button>
			</div>
		</div>
	{/if}
</Dialog>
