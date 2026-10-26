<script lang="ts">
	import { Trash2 } from 'lucide-svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import type { Student } from '$lib/api';

	let {
		deleteTarget,
		onConfirm,
		onCancel
	}: {
		deleteTarget: Student | null;
		onConfirm: () => void;
		onCancel: () => void;
	} = $props();
</script>

<Dialog
	open={!!deleteTarget}
	title="Delete student?"
	description={deleteTarget ? `${deleteTarget.name} will be permanently removed.` : ''}
	maxWidth="sm"
	onClose={onCancel}
>
	<div
		class="flex items-start gap-3 rounded-lg border border-destructive/25 bg-destructive/[0.04] p-3"
	>
		<div
			class="grid size-7 shrink-0 place-items-center rounded-full bg-destructive/10 text-destructive"
		>
			<Trash2 class="size-3.5" aria-hidden="true" strokeWidth={1.9} />
		</div>
		<p class="text-[12px] leading-5 text-muted-foreground">
			Attendance history for this learner stays in the logs under their name. Hold
			<span class="font-semibold text-foreground">Shift</span> when clicking delete to skip this confirmation.
		</p>
	</div>

	<div class="flex justify-end gap-2 border-t border-border pt-3">
		<button onclick={onCancel} class="btn btn-secondary">Cancel</button>
		<button onclick={onConfirm} class="btn btn-danger">Delete student</button>
	</div>
</Dialog>
