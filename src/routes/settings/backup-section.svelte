<script lang="ts">
	import { backupState } from './settings-state.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { fade } from 'svelte/transition';
	import {
		DatabaseBackup,
		RotateCcw,
		FolderOpen,
		Download,
		Upload,
		SlidersHorizontal,
		Minimize2,
		FileSpreadsheet
	} from 'lucide-svelte';
	import {
		backupKindLabel,
		formatBackupBytes,
		formatBackupTimestamp
	} from '$lib/features/settings/backup';
	import WipeDialog from './wipe-dialog.svelte';
	import ExportFormatDialog from './export-format-dialog.svelte';
</script>

<section class="order-4 space-y-5 rounded-2xl border border-border bg-card p-6">
	<!-- ── Header ─────────────────────────────────────────────────────────── -->
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="min-w-0">
			<h3 class="text-lg font-medium">Data Management</h3>
			<p class="mt-1 max-w-xl text-sm text-muted-foreground">
				{#if backupState.backupCardMode === 'simple'}
					Your data is stored locally. Back up your records anytime, or restore a previous backup.
				{:else}
					Your data is stored locally. Automatic SQLite backups protect students, classes,
					attendance records, settings, and SF2 workbook mappings. Backups are zip archives in your
					Documents folder, so a copy is one copy-paste away.
				{/if}
			</p>
		</div>
		<button
			onclick={() => backupState.toggleBackupCardMode()}
			aria-pressed={backupState.backupCardMode === 'full'}
			title={backupState.backupCardMode === 'full'
				? 'Show only the essential actions'
				: 'Show advanced backup options'}
			class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-3 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground"
		>
			{#if backupState.backupCardMode === 'full'}
				<Minimize2 class="size-4" aria-hidden="true" />
				Compact view
			{:else}
				<SlidersHorizontal class="size-4" aria-hidden="true" />
				Full details
			{/if}
		</button>
	</div>

	{#if backupState.backupStatus?.lastError}
		<div class="rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
			<div>{backupState.backupStatus.lastError}</div>
		</div>
	{/if}

	{#key backupState.backupCardMode}
		{#if backupState.backupCardMode === 'simple'}
			<!-- ── Compact view: just the essential actions ────────────────────── -->
			<div transition:fade={{ duration: 160 }} class="space-y-4">
				<div class="grid gap-3 sm:grid-cols-2">
					<button
						onclick={() => backupState.onCreateBackupNow()}
						disabled={backupState.backupBusy}
						class="group flex items-center gap-4 rounded-2xl border border-primary/35 bg-primary/5 p-5 text-left transition-all hover:border-primary/60 hover:bg-primary/10 active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-60"
					>
						<span
							class="grid size-11 shrink-0 place-items-center rounded-xl bg-primary text-primary-foreground shadow-sm transition-transform group-hover:scale-105"
						>
							{#if backupState.backupBusy}
								<Spinner />
							{:else}
								<DatabaseBackup class="size-5" aria-hidden="true" />
							{/if}
						</span>
						<span class="min-w-0">
							<span class="block text-sm font-semibold">
								{backupState.backupBusy ? 'Backing Up...' : 'Back Up Data'}
							</span>
							<span class="mt-0.5 block text-xs text-muted-foreground">
								Save a safety copy of your students, attendance & settings
							</span>
						</span>
					</button>

					<button
						onclick={() => backupState.onCreateWorkbooksBackupNow()}
						disabled={backupState.workbooksBackupBusy || backupState.backupBusy}
						class="group flex items-center gap-4 rounded-2xl border border-border bg-surface/40 p-5 text-left transition-all hover:border-primary/40 hover:bg-surface active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-60"
					>
						<span
							class="grid size-11 shrink-0 place-items-center rounded-xl border border-border bg-background text-muted-foreground transition-colors group-hover:text-foreground"
						>
							{#if backupState.workbooksBackupBusy}
								<Spinner />
							{:else}
								<FileSpreadsheet class="size-5" aria-hidden="true" />
							{/if}
						</span>
						<span class="min-w-0">
							<span class="block text-sm font-semibold">
								{backupState.workbooksBackupBusy
									? 'Backing Up Workbooks...'
									: 'Back Up Workbooks Now'}
							</span>
							<span class="mt-0.5 block text-xs text-muted-foreground">
								Save just the SF2 Excel files, without copying the database
							</span>
						</span>
					</button>

					<button
						onclick={() => backupState.onChooseRestoreBackup()}
						disabled={backupState.restoreChoosing || backupState.restoreBusy}
						class="group flex items-center gap-4 rounded-2xl border border-border bg-surface/40 p-5 text-left transition-all hover:border-primary/40 hover:bg-surface active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-60"
					>
						<span
							class="grid size-11 shrink-0 place-items-center rounded-xl border border-border bg-background text-muted-foreground transition-colors group-hover:text-foreground"
						>
							{#if backupState.restoreChoosing}
								<Spinner />
							{:else}
								<RotateCcw class="size-5" aria-hidden="true" />
							{/if}
						</span>
						<span class="min-w-0">
							<span class="block text-sm font-semibold">
								{backupState.restoreChoosing ? 'Checking...' : 'Restore Backup'}
							</span>
							<span class="mt-0.5 block text-xs text-muted-foreground">
								Restore a backup folder, including its SF2 workbooks
							</span>
						</span>
					</button>
				</div>

				<div
					class="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground"
				>
					<span class="inline-flex items-center gap-1.5">
						<span class="size-1.5 rounded-full bg-primary" aria-hidden="true"></span>
						Last backup:
						{formatBackupTimestamp(backupState.backupStatus?.lastBackupAt)}
					</span>
					<span class="font-mono text-[11px]">
						{backupState.backupStatus
							? `${backupState.backupStatus.backupCount} local backups · ${backupState.backupStatus.retentionLimit} kept`
							: '…'}
					</span>
				</div>
			</div>
		{:else}
			<!-- ── Full detail view ─────────────────────────────────────────────── -->
			<div transition:fade={{ duration: 160 }} class="space-y-5">
				<div class="flex flex-wrap gap-2">
					<button
						onclick={() => backupState.onCreateBackupNow()}
						disabled={backupState.backupBusy}
						class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
					>
						{#if backupState.backupBusy}
							<Spinner />
						{:else}
							<DatabaseBackup class="size-4" aria-hidden="true" />
						{/if}
						{backupState.backupBusy ? 'Backing Up...' : 'Back Up Now'}
					</button>
					<button
						onclick={() => backupState.onCreateWorkbooksBackupNow()}
						disabled={backupState.workbooksBackupBusy || backupState.backupBusy}
						class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
					>
						{#if backupState.workbooksBackupBusy}
							<Spinner />
						{:else}
							<FileSpreadsheet class="size-4" aria-hidden="true" />
						{/if}
						{backupState.workbooksBackupBusy ? 'Backing Up...' : 'Back Up Workbooks Now'}
					</button>
					<button
						onclick={() => backupState.onChooseRestoreBackup()}
						disabled={backupState.restoreChoosing || backupState.restoreBusy}
						class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
					>
						{#if backupState.restoreChoosing}
							<Spinner />
						{:else}
							<RotateCcw class="size-4" aria-hidden="true" />
						{/if}
						{backupState.restoreChoosing ? 'Checking...' : 'Restore Backup'}
					</button>
					<button
						onclick={() => backupState.onOpenBackupFolder()}
						disabled={backupState.backupFolderOpening}
						class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
					>
						{#if backupState.backupFolderOpening}
							<Spinner />
						{:else}
							<FolderOpen class="size-4" aria-hidden="true" />
						{/if}
						{backupState.backupFolderOpening ? 'Opening...' : 'Open Backup Folder'}
					</button>
				</div>

				<div class="grid gap-3 sm:grid-cols-2">
					<div class="rounded-xl border border-border bg-surface p-4">
						<div class="label-mono">Last Backup</div>
						<div class="mt-2 text-sm font-semibold">
							{formatBackupTimestamp(backupState.backupStatus?.lastBackupAt)}
						</div>
					</div>
					<div class="rounded-xl border border-border bg-surface p-4">
						<div class="label-mono">Stored Locally</div>
						<div class="mt-2 text-sm font-semibold">
							{backupState.backupStatus
								? `${backupState.backupStatus.backupCount} / ${backupState.backupStatus.retentionLimit}`
								: 'Loading'}
						</div>
					</div>
				</div>

				<div class="grid gap-3 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
					<div class="rounded-xl border border-border bg-background p-4">
						<div class="label-mono">Supported Backup Types</div>
						<div class="mt-3 grid gap-2 text-sm">
							<div class="rounded-md border border-border bg-surface px-3 py-2">
								<div class="font-semibold">SQLite full restore</div>
								<div class="mt-0.5 text-xs text-muted-foreground">
									Full database backup and restore, including SF2 mappings.
								</div>
							</div>
							<div class="rounded-md border border-border bg-surface px-3 py-2">
								<div class="font-semibold">SF2 workbook backups</div>
								<div class="mt-0.5 text-xs text-muted-foreground">
									Every backup folder also copies the SF2 Excel workbooks, with an X-mark count per
									file.
								</div>
							</div>
							<div class="rounded-md border border-border bg-surface px-3 py-2">
								<div class="font-semibold">JSON merge import</div>
								<div class="mt-0.5 text-xs text-muted-foreground">
									Merges students, attendance, classes, settings, and audit data.
								</div>
							</div>
							<div class="rounded-md border border-border bg-surface px-3 py-2">
								<div class="font-semibold">Local safety backups</div>
								<div class="mt-0.5 text-xs text-muted-foreground">
									Automatic, manual, pre-restore, and pre-update backup folders.
								</div>
							</div>
						</div>
					</div>

					<div class="rounded-xl border border-border bg-background p-4">
						<div class="flex items-center justify-between gap-3">
							<div class="label-mono">Latest Local Backups</div>
							<span class="font-mono text-xs text-muted-foreground">
								{backupState.backupSummaries.length} entries
							</span>
						</div>
						{#if backupState.backupSummaries.length === 0}
							<p class="mt-4 text-sm text-muted-foreground">No local backups found yet.</p>
						{:else}
							<div class="mt-3 max-h-44 space-y-2 overflow-auto pr-1">
								{#each backupState.backupSummaries.slice(0, 5) as backup (backup.path)}
									<div class="rounded-md border border-border bg-surface px-3 py-2 text-sm">
										<div class="flex items-start justify-between gap-3">
											<div class="min-w-0">
												<div class="truncate font-semibold">{backup.fileName}</div>
												<div class="mt-0.5 font-mono text-[11px] text-muted-foreground">
													{formatBackupTimestamp(backup.createdAt)} /
													{formatBackupBytes(backup.sizeBytes)}
												</div>
												{#if backup.workbookCount > 0}
													<div class="mt-0.5 font-mono text-[11px] text-muted-foreground">
														{backup.workbookCount} workbook(s) · {backup.totalXCount} X mark(s)
													</div>
												{/if}
											</div>
											<span
												class="shrink-0 rounded-pill border border-border bg-background px-2 py-1 font-mono text-[10px] font-bold text-primary"
											>
												{backupKindLabel(backup.kind)}
											</span>
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				</div>

				<div class="flex flex-wrap gap-2 border-t border-border pt-5">
					<button
						onclick={() => backupState.openExportDialog()}
						class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
					>
						<Download class="size-4" aria-hidden="true" />
						Export Data
					</button>
					<button
						onclick={() => backupState.fileInput?.click()}
						class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
					>
						<Upload class="size-4" aria-hidden="true" />
						Import JSON Merge
					</button>
					<input
						bind:this={backupState.fileInput}
						type="file"
						accept="application/json"
						class="hidden"
						onchange={(e) => backupState.handleFileChange(e)}
					/>
				</div>

				<div class="space-y-3 border-t border-border pt-5">
					<button
						onclick={() => backupState.onWipe()}
						class="inline-flex items-center gap-2 rounded-pill border border-destructive/40 px-4 py-2 text-sm font-medium text-destructive transition-colors hover:bg-destructive/10"
					>
						Wipe all data
					</button>
				</div>
			</div>
		{/if}
	{/key}
</section>

<WipeDialog
	bind:open={backupState.wipeTarget}
	onconfirm={async () => {
		await backupState.onWipeConfirm();
	}}
/>

<ExportFormatDialog
	bind:open={backupState.exportDialogOpen}
	bind:format={backupState.exportFormat}
	onexport={() => backupState.onExport()}
	onclose={() => (backupState.exportDialogOpen = false)}
/>
