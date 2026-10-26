<script lang="ts">
	import { backupState } from './settings-state.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import {
		ChevronDown,
		DatabaseBackup,
		Download,
		FileSpreadsheet,
		FolderOpen,
		RotateCcw,
		Upload
	} from 'lucide-svelte';
	import {
		backupKindLabel,
		formatBackupBytes,
		formatBackupTimestamp
	} from '$lib/features/settings/backup';
	import SettingsSection from './settings-section.svelte';
	import SettingsGroup from './settings-group.svelte';
	import SettingRow from './setting-row.svelte';
	import WipeDialog from './wipe-dialog.svelte';
	import ExportFormatDialog from './export-format-dialog.svelte';

	const secondaryButton = 'btn btn-secondary btn-sm';
	const summaryClass =
		'flex cursor-pointer list-none items-center justify-between px-4 py-3 text-[13px] font-medium text-foreground';
</script>

<SettingsSection
	title="Data & Backup"
	description="Everything is stored on this computer. Backups are zip archives in Documents/EES-AMS/backups."
>
	{#if backupState.backupStatus?.lastError}
		<div
			class="rounded-xl border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive"
		>
			{backupState.backupStatus.lastError}
		</div>
	{/if}

	<SettingsGroup>
		<SettingRow label="Back up data" description="Students, attendance, settings and SF2 mappings.">
			<button
				type="button"
				onclick={() => backupState.onCreateBackupNow()}
				disabled={backupState.backupBusy}
				class="btn btn-primary btn-sm"
			>
				{#if backupState.backupBusy}
					<Spinner />
				{:else}
					<DatabaseBackup class="size-3.5" aria-hidden="true" />
				{/if}
				{backupState.backupBusy ? 'Backing up…' : 'Back Up Now'}
			</button>
		</SettingRow>

		<SettingRow label="Back up workbooks" description="Just the SF2 Excel files.">
			<button
				type="button"
				onclick={() => backupState.onCreateWorkbooksBackupNow()}
				disabled={backupState.workbooksBackupBusy || backupState.backupBusy}
				class={secondaryButton}
			>
				{#if backupState.workbooksBackupBusy}
					<Spinner />
				{:else}
					<FileSpreadsheet class="size-3.5" aria-hidden="true" />
				{/if}
				{backupState.workbooksBackupBusy ? 'Backing up…' : 'Back Up'}
			</button>
		</SettingRow>

		<SettingRow label="Restore" description="Replace this computer's records with a backup.">
			<button
				type="button"
				onclick={() => backupState.onChooseRestoreBackup()}
				disabled={backupState.restoreChoosing || backupState.restoreBusy}
				class={secondaryButton}
			>
				{#if backupState.restoreChoosing}
					<Spinner />
				{:else}
					<RotateCcw class="size-3.5" aria-hidden="true" />
				{/if}
				{backupState.restoreChoosing ? 'Checking…' : 'Choose Backup…'}
			</button>
		</SettingRow>
	</SettingsGroup>

	<SettingsGroup>
		<SettingRow
			label="Last backup"
			description={backupState.backupStatus
				? `${formatBackupTimestamp(backupState.backupStatus.lastBackupAt)} · ${backupState.backupStatus.backupCount} of ${backupState.backupStatus.retentionLimit} kept`
				: 'Reading backup status…'}
		>
			<button
				type="button"
				onclick={() => backupState.onOpenBackupFolder()}
				disabled={backupState.backupFolderOpening}
				class={secondaryButton}
			>
				{#if backupState.backupFolderOpening}
					<Spinner />
				{:else}
					<FolderOpen class="size-3.5" aria-hidden="true" />
				{/if}
				Open Folder
			</button>
		</SettingRow>

		<details class="group">
			<summary class={summaryClass}>
				Advanced
				<ChevronDown
					class="size-4 text-muted-foreground transition-transform group-open:rotate-180"
					aria-hidden="true"
				/>
			</summary>
			<div class="divide-y divide-border border-t border-border">
				<SettingRow label="Export data" description="JSON, or the raw database file.">
					<button
						type="button"
						onclick={() => backupState.openExportDialog()}
						class={secondaryButton}
					>
						<Download class="size-3.5" aria-hidden="true" />
						Export…
					</button>
				</SettingRow>

				<SettingRow label="Import JSON merge" description="Merges an exported file into this data.">
					<button
						type="button"
						onclick={() => backupState.fileInput?.click()}
						class={secondaryButton}
					>
						<Upload class="size-3.5" aria-hidden="true" />
						Import…
					</button>
					<input
						bind:this={backupState.fileInput}
						type="file"
						accept="application/json"
						class="hidden"
						onchange={(e) => backupState.handleFileChange(e)}
					/>
				</SettingRow>

				<SettingRow
					label="Wipe all data"
					description="Deletes every record. A safety backup is written first."
					tone="danger"
				>
					<button type="button" onclick={() => backupState.onWipe()} class="btn btn-danger btn-sm">
						Wipe…
					</button>
				</SettingRow>
			</div>
		</details>
	</SettingsGroup>

	<SettingsGroup>
		<details class="group">
			<summary class={summaryClass}>
				Recent backups
				<span class="font-mono text-[11px] font-normal text-muted-foreground">
					{backupState.backupSummaries.length}
				</span>
			</summary>
			<div class="border-t border-border">
				{#if backupState.backupSummaries.length === 0}
					<p class="px-4 py-3 text-xs text-muted-foreground">No local backups found yet.</p>
				{:else}
					<div class="divide-y divide-border">
						{#each backupState.backupSummaries.slice(0, 6) as backup (backup.path)}
							<div class="flex items-baseline justify-between gap-3 px-4 py-2.5">
								<div class="min-w-0">
									<p class="truncate font-mono text-[11px]">{backup.fileName}</p>
									<p class="mt-0.5 font-mono text-[11px] text-muted-foreground">
										{formatBackupTimestamp(backup.createdAt)} · {formatBackupBytes(
											backup.sizeBytes
										)}
										{#if backup.workbookCount > 0}
											· {backup.workbookCount} workbook(s)
										{/if}
									</p>
								</div>
								<span class="shrink-0 text-[11px] text-muted-foreground">
									{backupKindLabel(backup.kind)}
								</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</details>
	</SettingsGroup>
</SettingsSection>

<WipeDialog
	bind:open={backupState.wipeTarget}
	busy={backupState.wipeBusy}
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
