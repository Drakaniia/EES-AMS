<script lang="ts">
	import { settingsState } from './settings-state.svelte.ts';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import {
		DatabaseBackup,
		RotateCcw,
		FolderOpen,
		CloudUpload,
		LogOut,
		Cloud,
		FolderSync,
		Trash2,
		Download,
		Upload
	} from 'lucide-svelte';
	import {
		backupKindLabel,
		backupPathLabel,
		formatBackupBytes,
		formatBackupTimestamp
	} from '$lib/features/settings/backup';
	import WipeDialog from './wipe-dialog.svelte';
	import ExportFormatDialog from './export-format-dialog.svelte';
</script>

<section class="order-4 space-y-5 rounded-2xl border border-border bg-card p-6">
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div>
			<h3 class="text-lg font-medium">Data Management</h3>
			<p class="mt-1 text-sm text-muted-foreground">
				Your data is stored locally. Automatic SQLite backups protect students, classes,
				attendance records, settings, and SF2 workbook mappings. Connect Google Drive with
				full Drive access to upload backups through browser sign-in, or use a local sync
				folder as a fallback.
			</p>
		</div>
		<div class="flex flex-wrap gap-2">
			<button
				onclick={() => settingsState.onCreateBackupNow()}
				disabled={settingsState.backupBusy}
				class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if settingsState.backupBusy}
					<Spinner />
				{:else}
					<DatabaseBackup class="size-4" aria-hidden="true" />
				{/if}
				{settingsState.backupBusy ? 'Backing Up...' : 'Back Up Now'}
			</button>
			<button
				onclick={() => settingsState.onChooseRestoreBackup()}
				disabled={settingsState.restoreChoosing || settingsState.restoreBusy}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if settingsState.restoreChoosing}
					<Spinner />
				{:else}
					<RotateCcw class="size-4" aria-hidden="true" />
				{/if}
				{settingsState.restoreChoosing ? 'Checking...' : 'Restore Backup'}
			</button>
			<button
				onclick={() => settingsState.onOpenBackupFolder()}
				disabled={settingsState.backupFolderOpening}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if settingsState.backupFolderOpening}
					<Spinner />
				{:else}
					<FolderOpen class="size-4" aria-hidden="true" />
				{/if}
				{settingsState.backupFolderOpening ? 'Opening...' : 'Open Backup Folder'}
			</button>
		</div>
	</div>

	<div class="grid gap-3 sm:grid-cols-4">
		<div class="rounded-xl border border-border bg-surface p-4">
			<div class="label-mono">Last Backup</div>
			<div class="mt-2 text-sm font-semibold">
				{formatBackupTimestamp(settingsState.backupStatus?.lastBackupAt)}
			</div>
		</div>
		<div class="rounded-xl border border-border bg-surface p-4">
			<div class="label-mono">Stored Locally</div>
			<div class="mt-2 text-sm font-semibold">
				{settingsState.backupStatus
					? `${settingsState.backupStatus.backupCount} / ${settingsState.backupStatus.retentionLimit}`
					: 'Loading'}
			</div>
		</div>
		<div class="rounded-xl border border-border bg-surface p-4">
			<div class="label-mono">Sync Folder</div>
			<div class="mt-2 text-sm font-semibold break-all">
				{backupPathLabel(settingsState.backupStatus?.syncFolderPath)}
			</div>
		</div>
		<div class="rounded-xl border border-border bg-surface p-4">
			<div class="label-mono">Google Drive</div>
			<div class="mt-2 text-sm font-semibold break-all">
				{settingsState.googleDriveStatusLabel()}
			</div>
			{#if settingsState.backupStatus?.lastGoogleDriveBackupAt}
				<div class="mt-1 text-xs text-muted-foreground">
					Last upload {formatBackupTimestamp(settingsState.backupStatus.lastGoogleDriveBackupAt)}
				</div>
			{/if}
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
					<div class="font-semibold">JSON merge import</div>
					<div class="mt-0.5 text-xs text-muted-foreground">
						Merges students, attendance, classes, settings, and audit data.
					</div>
				</div>
				<div class="rounded-md border border-border bg-surface px-3 py-2">
					<div class="font-semibold">Local safety backups</div>
					<div class="mt-0.5 text-xs text-muted-foreground">
						Automatic, manual, and pre-restore SQLite backup files.
					</div>
				</div>
			</div>
		</div>

		<div class="rounded-xl border border-border bg-background p-4">
			<div class="flex items-center justify-between gap-3">
				<div class="label-mono">Latest Local Backups</div>
				<span class="font-mono text-xs text-muted-foreground">
					{settingsState.backupSummaries.length} files
				</span>
			</div>
			{#if settingsState.backupSummaries.length === 0}
				<p class="mt-4 text-sm text-muted-foreground">No local backups found yet.</p>
			{:else}
				<div class="mt-3 max-h-44 space-y-2 overflow-auto pr-1">
					{#each settingsState.backupSummaries.slice(0, 5) as backup (backup.path)}
						<div class="rounded-md border border-border bg-surface px-3 py-2 text-sm">
							<div class="flex items-start justify-between gap-3">
								<div class="min-w-0">
									<div class="truncate font-semibold">{backup.fileName}</div>
									<div class="mt-0.5 font-mono text-[11px] text-muted-foreground">
										{formatBackupTimestamp(backup.createdAt)} / {formatBackupBytes(backup.sizeBytes)}
									</div>
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

	{#if settingsState.backupStatus?.lastError || settingsState.backupStatus?.lastSyncError || settingsState.backupStatus?.lastGoogleDriveError}
		<div
			class="rounded-xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900"
		>
			{#if settingsState.backupStatus.lastError}
				<div>{settingsState.backupStatus.lastError}</div>
			{/if}
			{#if settingsState.backupStatus.lastSyncError}
				<div>{settingsState.backupStatus.lastSyncError}</div>
			{/if}
			{#if settingsState.backupStatus.lastGoogleDriveError}
				<div>{settingsState.backupStatus.lastGoogleDriveError}</div>
			{/if}
		</div>
	{/if}

	<div class="flex flex-wrap gap-2 border-t border-border pt-5">
		{#if settingsState.backupStatus?.googleDriveConnected}
			<button
				onclick={() => settingsState.onUploadLatestBackupToGoogleDrive()}
				disabled={settingsState.googleDriveBusy}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if settingsState.googleDriveBusy}
					<Spinner />
				{:else}
					<CloudUpload class="size-4" aria-hidden="true" />
				{/if}
				Upload Latest to Drive
			</button>
			<button
				onclick={() => settingsState.onDisconnectGoogleDriveBackup()}
				disabled={settingsState.googleDriveBusy}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if settingsState.googleDriveBusy}
					<Spinner />
				{:else}
					<LogOut class="size-4" aria-hidden="true" />
				{/if}
				Disconnect Google Drive
			</button>
		{:else}
			<button
				onclick={() => settingsState.onConnectGoogleDriveBackup()}
				disabled={settingsState.googleDriveBusy || settingsState.backupStatus?.googleDriveConfigured === false}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				title={settingsState.backupStatus?.googleDriveConfigured === false
					? 'Set EES_AMS_GOOGLE_CLIENT_ID before building the app'
					: 'Open browser sign-in for full Google Drive access'}
			>
				{#if settingsState.googleDriveBusy}
					<Spinner />
				{:else}
					<Cloud class="size-4" aria-hidden="true" />
				{/if}
				{settingsState.googleDriveBusy ? 'Connecting...' : 'Connect Google Drive'}
			</button>
		{/if}
		<button
			onclick={() => settingsState.onChooseBackupSyncFolder()}
			disabled={settingsState.syncFolderBusy}
			class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
		>
			{#if settingsState.syncFolderBusy}
				<Spinner />
			{:else}
				<FolderSync class="size-4" aria-hidden="true" />
			{/if}
			Choose Local Sync Folder
		</button>
		<button
			onclick={() => settingsState.onClearBackupSyncFolder()}
			disabled={settingsState.syncFolderBusy || !settingsState.backupStatus?.syncFolderPath}
			class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
		>
			{#if settingsState.syncFolderBusy}
				<Spinner />
			{:else}
				<Trash2 class="size-4" aria-hidden="true" />
			{/if}
			Clear Sync Folder
		</button>
		<button
			onclick={() => settingsState.openExportDialog()}
			class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
		>
			<Download class="size-4" aria-hidden="true" />
			Export Data
		</button>
		<button
			onclick={() => settingsState.fileInput?.click()}
			class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
		>
			<Upload class="size-4" aria-hidden="true" />
			Import JSON Merge
		</button>
		<input
			bind:this={settingsState.fileInput}
			type="file"
			accept="application/json"
			class="hidden"
			onchange={(e) => settingsState.handleFileChange(e)}
		/>
	</div>

	<div class="space-y-3 border-t border-border pt-5">
		<button
			onclick={() => settingsState.onWipe()}
			class="inline-flex items-center gap-2 rounded-pill border border-destructive/40 px-4 py-2 text-sm font-medium text-destructive transition-colors hover:bg-destructive/10"
		>
			Wipe all data
		</button>
	</div>
</section>

<WipeDialog
	bind:open={settingsState.wipeTarget}
	onconfirm={async () => {
		await settingsState.onWipeConfirm();
	}}
/>

<ExportFormatDialog
	bind:open={settingsState.exportDialogOpen}
	bind:format={settingsState.exportFormat}
	onexport={() => settingsState.onExport()}
	onclose={() => (settingsState.exportDialogOpen = false)}
/>
