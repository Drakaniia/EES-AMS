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
		class="flex flex-wrap items-center justify-center gap-x-3 gap-y-1 border-b border-amber-500/30 bg-amber-50 px-4 py-2 text-[12px] leading-5 text-amber-900 dark:border-amber-500/25 dark:bg-amber-950/25 dark:text-amber-200"
	>
		<span
			><strong class="font-semibold">Temporary mode</strong> — changes won’t persist after close.</span
		>
		<button
			type="button"
			onclick={onRetryDatabase}
			disabled={databaseStatus.retrying}
			class="control-ring rounded-md border border-amber-600/35 bg-background px-2.5 py-1 text-[11.5px] font-semibold hover:bg-surface disabled:opacity-60"
		>
			{databaseStatus.retrying ? 'Retrying…' : 'Retry database'}
		</button>
		<a
			href={resolve('/settings')}
			class="control-ring rounded-md px-2 py-1 text-[11.5px] font-semibold underline underline-offset-2"
		>
			Back up now
		</a>
	</div>
{:else if resolved}
	<div
		role="alert"
		class="mx-auto flex w-full max-w-2xl flex-col gap-5 rounded-2xl border border-border bg-card p-6 text-left shadow-[var(--shadow-lifted)]"
	>
		<div class="flex items-start gap-3">
			<div
				class="grid size-9 shrink-0 place-items-center rounded-full bg-destructive/10 text-destructive"
				aria-hidden="true"
			>
				<TriangleAlert class="size-[18px]" strokeWidth={1.9} />
			</div>
			<div class="min-w-0 space-y-1">
				<h2 class="text-[16px] font-semibold tracking-[-0.015em] text-foreground">
					Attendance data can’t be opened yet
				</h2>
				<p class="text-[13px] leading-5 text-muted-foreground">{resolved.plain}</p>
			</div>
		</div>

		<ol class="space-y-1.5 text-[13px] leading-5 text-foreground">
			{#each resolved.steps as step, index (index)}
				<li class="flex gap-2.5">
					<span
						class="mt-0.5 grid size-4 shrink-0 place-items-center rounded-full bg-surface text-[10px] font-semibold text-muted-foreground"
					>
						{index + 1}
					</span>
					<span>{step}</span>
				</li>
			{/each}
		</ol>

		<div class="flex flex-wrap gap-2">
			<button
				type="button"
				onclick={() => void databaseStatus.retry()}
				disabled={databaseStatus.retrying}
				class="btn btn-primary"
			>
				{databaseStatus.retrying ? 'Retrying…' : 'Retry'}
			</button>
			<button
				type="button"
				onclick={() => void databaseStatus.enterTemporary()}
				disabled={databaseStatus.retrying}
				class="btn btn-secondary"
			>
				Continue without saving
			</button>
			<a href={resolve('/settings')} class="btn btn-secondary">Open backup &amp; restore</a>
		</div>

		<details class="rounded-lg border border-border bg-surface/60 p-3 text-[11.5px]">
			<summary class="cursor-pointer font-semibold text-muted-foreground">Technical details</summary
			>
			<p class="mt-2 font-mono leading-5 break-all whitespace-pre-wrap text-muted-foreground">
				{resolved.technical}
			</p>
			<button type="button" onclick={copyDetails} class="btn btn-secondary btn-sm mt-2">
				{copied ? 'Copied' : 'Copy details'}
			</button>
		</details>
	</div>
{/if}
