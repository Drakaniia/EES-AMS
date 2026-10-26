<script lang="ts">
	import { updateStore } from '$lib/stores/update.svelte';
	import { openExternalUrl } from '$lib/features/settings/update';
	import { updateSectionState } from './update-state.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import SettingsSection from './settings-section.svelte';
	import SettingsGroup from './settings-group.svelte';
	import SettingRow from './setting-row.svelte';
	import {
		AlertTriangle,
		CheckCircle2,
		Download,
		ExternalLink,
		RefreshCw,
		RotateCcw,
		TriangleAlert
	} from 'lucide-svelte';

	const RELEASE_BASE = 'https://github.com/Drakaniia/EES-AMS/releases/tag/app-v';

	// The failure text is deliberately not line-clamped: the reason a backup or
	// install was refused sits at the end of the message chain ("created backup
	// failed validation: Backup schema version 22 is newer than this app
	// supports (18)"), not in the first two lines.

	const isBusy = $derived(
		updateStore.status === 'checking' || updateStore.status === 'downloading'
	);

	const progressPercent = $derived.by(() => {
		const { progress } = updateStore;
		if (!progress?.total) return 0;
		return Math.min(100, Math.round((progress.downloaded / progress.total) * 100));
	});

	const secondaryButton = 'btn btn-secondary btn-sm';
	const primaryButton = 'btn btn-primary btn-sm';

	function formatBytes(bytes: number): string {
		if (bytes <= 0) return '0 MB';
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function viewRelease(version: string) {
		void openExternalUrl(`${RELEASE_BASE}${version}`);
	}
</script>

<SettingsSection
	title="Software Update"
	description="Updates are downloaded from GitHub releases and installed on restart."
>
	{#if updateStore.attendanceWarning}
		<div
			class="rounded-xl border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive"
		>
			<div class="flex items-center gap-1.5 font-semibold">
				<TriangleAlert class="size-3.5 shrink-0" aria-hidden="true" />
				Attendance records went missing
			</div>
			<p class="mt-1">{updateStore.attendanceWarning}</p>
			<p class="mt-1">Review your backups in Data &amp; Backup before recording more attendance.</p>
		</div>
	{/if}

	<SettingsGroup>
		<SettingRow
			label="Version"
			description={updateStore.status === 'upToDate'
				? "You're up to date"
				: 'Check GitHub for a newer release.'}
		>
			<span class="font-mono text-sm text-muted-foreground">
				{updateStore.currentVersion ? `v${updateStore.currentVersion}` : '—'}
			</span>
			<button
				type="button"
				onclick={() => updateStore.refresh()}
				disabled={isBusy}
				class={secondaryButton}
				title="Check for updates"
			>
				<RefreshCw class="size-3.5" aria-hidden="true" />
				Check Now
			</button>
		</SettingRow>

		{#if updateStore.status === 'checking'}
			<div class="flex items-center gap-2 px-4 py-3 text-xs text-muted-foreground">
				<Spinner />
				Checking for updates…
			</div>
		{:else if updateStore.status === 'upToDate'}
			<div class="flex items-center gap-2 px-4 py-3 text-xs text-emerald-700 dark:text-emerald-400">
				<CheckCircle2 class="size-3.5 shrink-0" aria-hidden="true" />
				No updates available.
			</div>
		{:else if updateStore.status === 'available'}
			{@const info = updateStore.updateInfo}
			{#if info}
				<div class="space-y-3 bg-surface/40 px-4 py-3">
					<div class="flex items-center gap-2 text-[13px] font-medium">
						<Download class="size-3.5 shrink-0 text-primary" aria-hidden="true" />
						Version {info.version} is available
					</div>
					{#if info.notes}
						<p class="line-clamp-3 text-xs leading-5 whitespace-pre-line text-muted-foreground">
							{info.notes}
						</p>
					{/if}
					<div class="flex items-center gap-2">
						<button type="button" onclick={() => updateStore.download()} class={primaryButton}>
							Download Update
						</button>
						{#if info.version}
							{@const releaseVersion = info.version}
							<button
								type="button"
								onclick={() => viewRelease(releaseVersion)}
								class="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground hover:underline"
							>
								View on GitHub
								<ExternalLink class="size-3" aria-hidden="true" />
							</button>
						{/if}
					</div>
				</div>
			{/if}
		{:else if updateStore.status === 'downloading'}
			<div class="space-y-2 bg-surface/40 px-4 py-3">
				<div class="flex items-center justify-between gap-2 text-xs text-muted-foreground">
					<span class="inline-flex items-center gap-2">
						<Spinner />
						Downloading update…
					</span>
					<span class="font-mono">{progressPercent}%</span>
				</div>
				<div
					class="progress-track"
					role="progressbar"
					aria-valuemin="0"
					aria-valuemax="100"
					aria-valuenow={progressPercent}
					aria-label="Update download progress"
				>
					<div class="progress-fill" style={`width: ${progressPercent}%`}></div>
				</div>
				<div class="flex items-center justify-between gap-2 text-xs text-muted-foreground">
					<span class="font-mono">
						{updateStore.progress ? formatBytes(updateStore.progress.downloaded) : '0 MB'}
						{updateStore.progress?.total ? ` / ${formatBytes(updateStore.progress.total)}` : ''}
					</span>
					<button
						type="button"
						onclick={() => updateStore.cancel()}
						class="font-medium underline hover:text-foreground"
					>
						Cancel
					</button>
				</div>
			</div>
		{:else if updateStore.status === 'readyToRestart'}
			{@const version = updateStore.stagedVersion}
			<div class="space-y-3 bg-surface/40 px-4 py-3">
				<div class="flex items-center gap-2 text-[13px] font-medium">
					<RotateCcw class="size-3.5 shrink-0 text-primary" aria-hidden="true" />
					Version {version} is downloaded
				</div>
				<p class="text-xs leading-5 text-muted-foreground">
					Restart to apply it. The app relaunches automatically after installing.
				</p>
				{#if updateStore.stagedNotes}
					<p class="line-clamp-3 text-xs leading-5 whitespace-pre-line text-muted-foreground">
						{updateStore.stagedNotes}
					</p>
				{/if}
				<div class="flex gap-2">
					<button
						type="button"
						onclick={() => updateSectionState.onRestartRequested()}
						class={primaryButton}
					>
						Restart to Update
					</button>
					<button type="button" onclick={() => updateStore.later()} class={secondaryButton}>
						Later
					</button>
				</div>
			</div>
		{:else if updateStore.status === 'deferred'}
			<div class="flex items-center justify-between gap-3 bg-surface/40 px-4 py-3">
				<p class="min-w-0 truncate text-xs text-muted-foreground">
					Update v{updateStore.stagedVersion} ready — restart to apply
				</p>
				<button
					type="button"
					onclick={() => updateSectionState.onRestartRequested()}
					class="shrink-0 text-xs font-medium text-primary hover:underline"
				>
					Restart
				</button>
			</div>
		{:else if updateStore.status === 'failed'}
			<div class="space-y-2 bg-destructive/5 px-4 py-3">
				<div class="flex items-center gap-1.5 text-[13px] font-medium text-destructive">
					<AlertTriangle class="size-3.5 shrink-0" aria-hidden="true" />
					{updateStore.failedStage === 'download'
						? 'Download failed'
						: updateStore.failedStage === 'install'
							? "Update couldn't be installed"
							: "Couldn't check for updates"}
				</div>
				{#if updateStore.error}
					<p class="text-xs break-words text-muted-foreground">{updateStore.error}</p>
				{/if}
				<button type="button" onclick={() => updateStore.retry()} class={secondaryButton}>
					Retry
				</button>
			</div>
		{/if}
	</SettingsGroup>
</SettingsSection>

<Dialog
	open={updateSectionState.restartConfirmOpen}
	title="Restart to Update?"
	description="You have unsaved changes in General. They will be discarded when the app restarts."
	onClose={() => (updateSectionState.restartConfirmOpen = false)}
>
	<div class="flex justify-end gap-3">
		<button
			type="button"
			onclick={() => (updateSectionState.restartConfirmOpen = false)}
			class="btn btn-secondary"
		>
			Cancel
		</button>
		<button
			type="button"
			onclick={() => updateSectionState.confirmRestart()}
			class="btn btn-primary"
		>
			Restart Anyway
		</button>
	</div>
</Dialog>
