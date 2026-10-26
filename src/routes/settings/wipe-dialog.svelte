<script lang="ts">
	import { Trash2 } from 'lucide-svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';

	let {
		open = $bindable(false),
		busy = false,
		onconfirm
	}: {
		open?: boolean;
		busy?: boolean;
		onconfirm?: () => Promise<void>;
	} = $props();
</script>

<Dialog
	{open}
	title="Erase all data?"
	description="This permanently deletes every student, attendance event, class and setting."
	maxWidth="sm"
	onClose={() => !busy && (open = false)}
>
	<div
		class="flex items-start gap-3 rounded-lg border border-destructive/25 bg-destructive/[0.04] p-3"
	>
		<div
			class="grid size-8 shrink-0 place-items-center rounded-full bg-destructive/10 text-destructive"
		>
			<Trash2 class="size-4" aria-hidden="true" strokeWidth={1.9} />
		</div>
		<p class="text-[12px] leading-5 text-muted-foreground">
			A safety backup is written first, but restoring from it is a manual step. This action cannot
			be undone in the app.
		</p>
	</div>

	<div class="flex justify-end gap-2 border-t border-border pt-3">
		<button onclick={() => (open = false)} disabled={busy} class="btn btn-secondary">Cancel</button>
		<button
			onclick={async () => {
				await onconfirm?.();
				open = false;
			}}
			disabled={busy}
			class="btn btn-danger"
		>
			{#if busy}
				<Spinner />
				Wiping…
			{:else}
				Wipe all data
			{/if}
		</button>
	</div>
</Dialog>
