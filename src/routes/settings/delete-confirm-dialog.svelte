<script lang="ts">
	import Spinner from '$lib/components/ui/Spinner.svelte';

	let {
		open = $bindable(false),
		target = $bindable<{ id: string; name: string } | null>(null),
		deleting = $bindable(false),
		onconfirm
	}: {
		open?: boolean;
		target?: { id: string; name: string } | null;
		deleting?: boolean;
		onconfirm?: () => void;
	} = $props();
</script>

{#if open && target}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (open = false)}
		onkeydown={(e) => e.key === 'Escape' && (open = false)}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="delete-class-dialog-title"
	>
		<div class="w-full max-w-sm space-y-5 rounded-2xl border border-border bg-background p-6">
			<div class="flex flex-col items-center gap-3 text-center">
				<div class="flex size-12 items-center justify-center rounded-full bg-destructive/10">
					<svg
						class="size-6 text-destructive"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="3 6 5 6 21 6" />
						<path
							d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"
						/>
					</svg>
				</div>
				<div class="w-full text-left">
					<h2 id="delete-class-dialog-title" class="text-lg font-semibold">Delete class?</h2>
					<p class="mt-1 text-sm text-muted-foreground">
						<span class="font-medium text-foreground">{target.name}</span> will be permanently removed.
						Students will remain but will be unassigned.
					</p>
					<p class="mt-4 text-xs leading-relaxed text-muted-foreground">
						<strong class="font-semibold text-accent">PROTIP:</strong>
						<span class="block">
							You can hold down <strong class="font-semibold">Shift</strong> when clicking the delete
							button to bypass this confirmation entirely.
						</span>
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (open = false)}
					class="flex-1 rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={onconfirm}
					disabled={deleting}
					class="inline-flex flex-1 items-center justify-center gap-2 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if deleting}
						<Spinner />
					{/if}
					{deleting ? 'Deleting...' : 'Delete'}
				</button>
			</div>
		</div>
	</div>
{/if}
