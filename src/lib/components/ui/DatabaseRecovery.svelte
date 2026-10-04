<script lang="ts">
	import { resolve } from '$app/paths';
	import { TriangleAlert } from 'lucide-svelte';
	import { summarizeUnavailableError } from '$lib/db';
	import { databaseStatus } from '$lib/stores/database-status.svelte';

	type Props = {
		/** The raw worker detail (with the `[cause=…]` token). Falls back to the store's. */
		detail?: string | null;
		/** One-line banner for temporary mode instead of the full card. */
		compact?: boolean;
	};

	let { detail = null, compact = false }: Props = $props();

	const effectiveDetail = $derived(detail ?? databaseStatus.detail);
	const resolved = $derived(effectiveDetail ? summarizeUnavailableError(effectiveDetail) : null);

	let copied = $state(false);

	async function copyDetails() {
		if (!resolved) return;
		try {
			await navigator.clipboard.writeText(resolved.technical);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		} catch {
			copied = false;
		}
	}

	async function onRetryDatabase() {
		if (databaseStatus.state === 'temporary') {
			if (await databaseStatus.hasTemporaryData()) {
				const keepGoing = confirm(
					'Temporary data will be discarded when the real database reopens. Take a backup first via Settings → Data Management, unless you already have. Continue anyway?'
				);
				if (!keepGoing) return;
			}
		}
		await databaseStatus.retry();
	}
</script>

{#if compact}
	<div
		role="status"
		class="flex flex-wrap items-center justify-center gap-x-3 gap-y-1 border-b border-amber-500/40 bg-amber-50 px-4 py-2 text-xs leading-5 text-amber-900"
	>
		<span><strong>Temporary mode</strong> — changes won’t persist after close.</span>
		<button
			type="button"
			onclick={onRetryDatabase}
			disabled={databaseStatus.retrying}
			class="control-ring rounded-pill border border-amber-600/40 bg-background px-3 py-1 text-xs font-semibold hover:bg-surface disabled:opacity-60"
		>
			{databaseStatus.retrying ? 'Retrying…' : 'Retry database'}
		</button>
		<a
			href={resolve('/settings')}
			class="control-ring rounded-pill px-2 py-1 text-xs font-semibold underline underline-offset-2"
		>
			Back up now
		</a>
	</div>
{:else if resolved}
	<div
		role="alert"
		class="mx-auto flex w-full max-w-2xl flex-col gap-4 rounded-2xl border border-border bg-surface p-6 text-left shadow-xl"
	>
		<div class="flex items-start gap-3">
			<div
				class="grid size-10 shrink-0 place-items-center rounded-xl border border-destructive/20 bg-destructive/10 text-destructive"
				aria-hidden="true"
			>
				<TriangleAlert class="size-5" />
			</div>
			<div class="min-w-0 space-y-1">
				<h2 class="text-base font-bold text-foreground">Attendance data can’t be opened yet</h2>
				<p class="text-sm leading-6 text-muted-foreground">{resolved.plain}</p>
			</div>
		</div>

		<ol class="list-decimal space-y-1 pl-5 text-sm leading-6 text-foreground">
			{#each resolved.steps as step, index (index)}
				<li>{step}</li>
			{/each}
		</ol>

		<div class="flex flex-wrap gap-2">
			<button
				type="button"
				onclick={() => void databaseStatus.retry()}
				disabled={databaseStatus.retrying}
				class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface disabled:opacity-60"
			>
				{databaseStatus.retrying ? 'Retrying…' : 'Retry'}
			</button>
			<button
				type="button"
				onclick={() => void databaseStatus.enterTemporary()}
				disabled={databaseStatus.retrying}
				class="control-ring rounded-pill bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground hover:bg-accent disabled:opacity-60"
			>
				Continue without saving (temporary)
			</button>
			<a
				href={resolve('/settings')}
				class="control-ring inline-flex items-center rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
			>
				Open backup &amp; restore
			</a>
		</div>

		<details class="rounded-xl border border-border bg-background p-3 text-xs">
			<summary class="cursor-pointer font-semibold text-muted-foreground">Details</summary>
			<p class="mt-2 font-mono leading-5 break-all whitespace-pre-wrap text-muted-foreground">
				{resolved.technical}
			</p>
			<button
				type="button"
				onclick={copyDetails}
				class="control-ring mt-2 rounded-pill border border-border px-3 py-1 text-xs font-medium hover:bg-surface"
			>
				{copied ? 'Copied' : 'Copy details'}
			</button>
		</details>
	</div>
{/if}
