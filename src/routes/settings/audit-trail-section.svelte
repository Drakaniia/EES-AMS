<script lang="ts">
	import { settingsState } from './settings-state.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { Trash2, History } from 'lucide-svelte';
	import {
		auditEntityLabel,
		auditMetadataPreview,
		formatAuditTimestamp
	} from '$lib/features/settings/backup';
</script>

<section class="space-y-4 rounded-2xl border border-border bg-card p-6">
	<div class="flex items-start justify-between gap-3">
		<div class="space-y-1">
			<h3 class="text-lg font-medium">Audit Trail</h3>
			<p class="text-xs text-muted-foreground">Latest accountability events.</p>
		</div>
		<div class="flex items-center gap-2">
			<button
				type="button"
				onclick={() => (settingsState.auditClearTarget = true)}
				disabled={settingsState.auditLoading ||
					settingsState.auditClearing ||
					settingsState.auditEvents.length === 0}
				class="inline-flex h-9 items-center gap-2 rounded-md border border-destructive/40 bg-background px-3 text-xs font-medium text-destructive transition-colors hover:bg-destructive/10 disabled:cursor-not-allowed disabled:opacity-60"
				title="Clear audit trail"
			>
				{#if settingsState.auditClearing}
					<Spinner />
				{:else}
					<Trash2 class="size-4" aria-hidden="true" />
				{/if}
				Clear
			</button>
			<button
				type="button"
				onclick={() => settingsState.reloadAuditEvents()}
				disabled={settingsState.auditLoading || settingsState.auditClearing}
				class="inline-flex size-9 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				title="Refresh audit trail"
			>
				{#if settingsState.auditLoading}
					<Spinner />
				{:else}
					<History class="size-4" aria-hidden="true" />
				{/if}
			</button>
		</div>
	</div>

	{#if settingsState.auditLoading && settingsState.auditEvents.length === 0}
		<div class="rounded-xl border border-border bg-surface p-4 text-sm text-muted-foreground">
			Loading audit trail...
		</div>
	{:else if settingsState.auditEvents.length === 0}
		<div class="rounded-xl border border-border bg-surface p-4 text-sm text-muted-foreground">
			No audit events recorded.
		</div>
	{:else}
		<div
			class="max-h-[28rem] divide-y divide-border overflow-y-auto rounded-xl border border-border"
		>
			{#each settingsState.auditEvents as event (event.id)}
				<div class="space-y-2 bg-background p-4">
					<div class="flex items-start justify-between gap-3">
						<div class="min-w-0">
							<div class="text-sm font-medium">{event.summary}</div>
							<div class="mt-1 text-xs text-muted-foreground">
								{formatAuditTimestamp(event.createdAt)} · {event.actor} · {auditEntityLabel(event)}
							</div>
						</div>
						<span
							class="shrink-0 rounded-md bg-surface px-2 py-1 text-[10px] font-semibold tracking-wide text-muted-foreground uppercase"
						>
							{event.action}
						</span>
					</div>
					{#if auditMetadataPreview(event)}
						<div class="text-xs text-muted-foreground">{auditMetadataPreview(event)}</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</section>

<!-- ── Audit clear confirmation ── -->
{#if settingsState.auditClearTarget}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => {
			if (!settingsState.auditClearing) settingsState.auditClearTarget = false;
		}}
		onkeydown={(e) =>
			e.key === 'Escape' &&
			!settingsState.auditClearing &&
			(settingsState.auditClearTarget = false)}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="audit-clear-dialog-title"
	>
		<div class="w-full max-w-sm space-y-5 rounded-2xl border border-border bg-background p-6">
			<div class="flex flex-col items-center gap-3 text-center">
				<div class="flex size-12 items-center justify-center rounded-full bg-destructive/10">
					<Trash2 class="size-6 text-destructive" aria-hidden="true" />
				</div>
				<div>
					<h2 id="audit-clear-dialog-title" class="text-lg font-semibold">Clear audit trail?</h2>
					<p class="mt-1 text-sm text-muted-foreground">
						This will permanently remove all Settings audit trail events.
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (settingsState.auditClearTarget = false)}
					disabled={settingsState.auditClearing}
					class="flex-1 rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					Cancel
				</button>
				<button
					onclick={() => settingsState.confirmClearAuditEvents()}
					disabled={settingsState.auditClearing}
					class="inline-flex flex-1 items-center justify-center gap-2 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if settingsState.auditClearing}
						<Spinner />
					{/if}
					{settingsState.auditClearing ? 'Clearing...' : 'Clear'}
				</button>
			</div>
		</div>
	</div>
{/if}
