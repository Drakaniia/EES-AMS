<script lang="ts">
	import { updateStore } from '$lib/stores/update.svelte';
	import { openExternalUrl } from '$lib/features/settings/update';
	import { updateSectionState } from './update-state.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import {
		AlertTriangle,
		CheckCircle2,
		Download,
		ExternalLink,
		RefreshCw,
		RotateCcw
	} from 'lucide-svelte';

	const RELEASE_BASE = 'https://github.com/Drakaniia/EES-AMS/releases/tag/app-v';

	const isBusy = $derived(
		updateStore.status === 'checking' || updateStore.status === 'downloading'
	);

	const progressPercent = $derived.by(() => {
		const { progress } = updateStore;
		if (!progress?.total) return 0;
		return Math.min(100, Math.round((progress.downloaded / progress.total) * 100));
	});

	function formatBytes(bytes: number): string {
		if (bytes <= 0) return '0 MB';
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function viewRelease(version: string) {
		void openExternalUrl(`${RELEASE_BASE}${version}`);
	}
</script>

<section class="space-y-5 rounded-2xl border border-border bg-card p-6">
	<div class="space-y-1">
		<h3 class="text-lg font-medium">Software Update</h3>
		<p class="text-xs text-muted-foreground">Check for and install new releases.</p>
	</div>

	<div class="space-y-4">
		<div class="flex items-center justify-between gap-2">
			<div class="min-w-0">
				<p class="label-mono text-xs text-muted-foreground">Current version</p>
				<p class="mt-0.5 text-sm font-medium">
					{updateStore.currentVersion ? `v${updateStore.currentVersion}` : '—'}
				</p>
			</div>
			<button
				type="button"
				onclick={() => updateStore.refresh()}
				disabled={isBusy}
				class="control-ring inline-grid size-9 shrink-0 place-items-center rounded-lg text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
				title="Check for updates"
				aria-label="Check for updates"
			>
				<RefreshCw class="size-4" aria-hidden="true" />
			</button>
		</div>

		{#if updateStore.status === 'checking'}
			<div class="flex items-center gap-2 text-sm text-muted-foreground">
				<Spinner /> Checking for updates…
			</div>
		{:else if updateStore.status === 'upToDate'}
			<div class="flex items-center gap-2 text-sm text-emerald-700">
				<CheckCircle2 class="size-4 shrink-0" aria-hidden="true" />
				You're up to date
			</div>
		{:else if updateStore.status === 'available'}
			{@const info = updateStore.updateInfo}
			{#if info}
				<div class="space-y-3 rounded-xl border border-orange-200 bg-orange-50 p-3">
					<div class="flex items-center gap-2 text-sm font-medium text-stone-900">
						<Download class="size-4 shrink-0 text-orange-600" aria-hidden="true" />
						Version {info.version} is available
					</div>
					{#if info.notes}
						<p class="line-clamp-3 text-xs whitespace-pre-line text-muted-foreground">
							{info.notes}
						</p>
					{/if}
					{#if info.version}
						{@const releaseVersion = info.version}
						<button
							type="button"
							onclick={() => viewRelease(releaseVersion)}
							class="inline-flex items-center gap-1 text-xs font-medium text-orange-700 hover:underline"
						>
							View on GitHub
							<ExternalLink class="size-3" aria-hidden="true" />
						</button>
					{/if}
					<button
						type="button"
						onclick={() => updateStore.download()}
						class="inline-flex w-full items-center justify-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
					>
						Download Update
					</button>
				</div>
			{/if}
		{:else if updateStore.status === 'downloading'}
			<div class="space-y-2">
				<div class="flex items-center justify-between gap-2 text-xs text-muted-foreground">
					<span class="inline-flex items-center gap-2">
						<Spinner /> Downloading update…
					</span>
					<span class="font-mono">{progressPercent}%</span>
				</div>
				<div
					class="h-2 overflow-hidden rounded-pill border border-primary/20 bg-background"
					role="progressbar"
					aria-valuemin="0"
					aria-valuemax="100"
					aria-valuenow={progressPercent}
					aria-label="Update download progress"
				>
					<div
						class="h-full rounded-pill bg-primary transition-[width]"
						style={`width: ${progressPercent}%`}
					></div>
				</div>
				<div class="flex items-center justify-between gap-2 text-xs text-muted-foreground">
					<span class="font-mono">
						{updateStore.progress ? formatBytes(updateStore.progress.downloaded) : '0 MB'}
						{updateStore.progress?.total ? ` / ${formatBytes(updateStore.progress.total)}` : ''}
					</span>
					<button
						type="button"
						onclick={() => updateStore.cancel()}
						class="font-medium text-muted-foreground underline hover:text-foreground"
					>
						Cancel
					</button>
				</div>
			</div>
		{:else if updateStore.status === 'readyToRestart'}
			{@const version = updateStore.stagedVersion}
			<div class="space-y-3 rounded-xl border border-orange-200 bg-orange-50 p-3">
				<div class="flex items-center gap-2 text-sm font-medium text-stone-900">
					<RotateCcw class="size-4 shrink-0 text-orange-600" aria-hidden="true" />
					Version {version} is downloaded
				</div>
				<p class="text-xs text-muted-foreground">
					Restart the app to apply the update. It will relaunch automatically after installing.
				</p>
				{#if updateStore.stagedNotes}
					<p class="line-clamp-3 text-xs whitespace-pre-line text-muted-foreground">
						{updateStore.stagedNotes}
					</p>
				{/if}
				<div class="flex gap-2">
					<button
						type="button"
						onclick={() => updateSectionState.onRestartRequested()}
						class="inline-flex flex-1 items-center justify-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
					>
						Restart to Update
					</button>
					<button
						type="button"
						onclick={() => updateStore.later()}
						class="inline-flex flex-1 items-center justify-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground"
					>
						Update Later
					</button>
				</div>
			</div>
		{:else if updateStore.status === 'deferred'}
			<div
				class="flex items-center justify-between gap-2 rounded-xl border border-orange-200 bg-orange-50/60 px-3 py-2"
			>
				<p class="min-w-0 truncate text-xs text-muted-foreground">
					Update v{updateStore.stagedVersion} ready — restart to apply
				</p>
				<button
					type="button"
					onclick={() => updateSectionState.onRestartRequested()}
					class="shrink-0 text-xs font-medium text-orange-700 hover:underline"
				>
					Restart
				</button>
			</div>
		{:else if updateStore.status === 'failed'}
			<div class="space-y-2 rounded-xl border border-red-200 bg-red-50 p-3">
				<div class="flex items-center gap-2 text-sm font-medium text-red-800">
					<AlertTriangle class="size-4 shrink-0 text-red-600" aria-hidden="true" />
					{updateStore.failedStage === 'download'
						? 'Download failed'
						: updateStore.failedStage === 'install'
							? "Update couldn't be installed"
							: "Couldn't check for updates"}
				</div>
				{#if updateStore.error}
					<p class="line-clamp-2 text-xs text-muted-foreground">{updateStore.error}</p>
				{/if}
				<button
					type="button"
					onclick={() => updateStore.retry()}
					class="control-ring inline-flex items-center justify-center gap-2 rounded-pill border border-border bg-background px-4 py-1.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground"
				>
					Retry
				</button>
			</div>
		{/if}
	</div>
</section>

<Dialog
	open={updateSectionState.restartConfirmOpen}
	title="Restart to Update?"
	description="You have unsaved changes in Global Settings. They will be discarded when the app restarts."
	onClose={() => (updateSectionState.restartConfirmOpen = false)}
>
	<div class="flex justify-end gap-3">
		<button
			type="button"
			onclick={() => (updateSectionState.restartConfirmOpen = false)}
			class="control-ring inline-flex items-center justify-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground"
		>
			Cancel
		</button>
		<button
			type="button"
			onclick={() => updateSectionState.confirmRestart()}
			class="inline-flex items-center justify-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
		>
			Restart Anyway
		</button>
	</div>
</Dialog>
