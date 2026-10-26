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

			<dl
				class="grid grid-cols-2 gap-x-4 gap-y-2 rounded-xl border border-border bg-surface/60 p-4 sm:grid-cols-3"
			>
				{@render metric('Students', String(backupState.restorePreview.studentCount))}
				{@render metric('Classes', String(backupState.restorePreview.classCount))}
				{@render metric('Attendance', String(backupState.restorePreview.eventCount))}
				{@render metric('Absences', String(backupState.restorePreview.absentCount))}
				{@render metric('Settings', String(backupState.restorePreview.settingsCount))}
				{@render metric('Size', formatBackupBytes(backupState.restorePreview.sizeBytes))}
			</dl>

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

			<div
				class="rounded-xl border border-amber-500/30 bg-amber-50/80 p-3.5 text-[12.5px] leading-5 text-amber-900 dark:border-amber-400/25 dark:bg-amber-950/25 dark:text-amber-200"
			>
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
					class="btn btn-secondary"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={() => backupState.onConfirmRestoreBackup()}
					disabled={backupState.restoreBusy}
					class="btn btn-danger"
				>
					{#if backupState.restoreBusy}
						<Spinner />
					{/if}
					{backupState.restoreBusy ? 'Restoring…' : 'Restore backup'}
				</button>
			</div>
		</div>
	{/if}
</Dialog>

{#snippet metric(label: string, value: string)}
	<div>
		<dt class="text-[11px] text-muted-foreground">{label}</dt>
		<dd class="mt-0.5 text-[14px] font-semibold tabular-nums">{value}</dd>
	</div>
{/snippet}
