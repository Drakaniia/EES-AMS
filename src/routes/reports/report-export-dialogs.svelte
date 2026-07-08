<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import TaskProgress from '$lib/components/ui/TaskProgress.svelte';
	import { Save } from 'lucide-svelte';
	import type { Sf2ExportPreview } from '$lib/db-rust';

	let {
		exportDialogOpen = $bindable(false),
		exportLoadingOpen = $bindable(false),
		preview,
		exporting,
		onConfirmExport
	}: {
		exportDialogOpen: boolean;
		exportLoadingOpen: boolean;
		preview: Sf2ExportPreview | null;
		exporting: boolean;
		onConfirmExport: () => void;
	} = $props();

	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	export function showToast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 4000);
	}
</script>

<Dialog
	open={exportDialogOpen}
	title="Confirm SF2 Export"
	description="Export copies the reviewed SF2 working workbook to your chosen file path."
	maxWidth="2xl"
	onClose={() => (exportDialogOpen = false)}
>
	<div class="grid gap-3 sm:grid-cols-2">
		<div class="rounded-md border border-border bg-surface p-3">
			<div class="label-mono">Mapped learners</div>
			<div class="mt-2 text-2xl font-semibold">{preview?.mappedStudents ?? 0}</div>
		</div>
		<div class="rounded-md border border-border bg-surface p-3">
			<div class="label-mono">Absences</div>
			<div class="mt-2 text-2xl font-semibold">{preview?.absenceCount ?? 0}</div>
		</div>
	</div>

	{#if preview && preview.warnings.length > 0}
		<div class="rounded-md border border-amber-500/30 bg-amber-50 p-4 text-sm text-amber-900">
			<div class="font-semibold">Review these warnings before exporting.</div>
			<ul class="mt-3 max-h-48 space-y-2 overflow-auto">
				{#each preview.warnings as warning, index (`confirm-warning-${index}-${warning}`)}
					<li>{warning}</li>
				{/each}
			</ul>
		</div>
	{:else}
		<div class="rounded-md border border-emerald-500/30 bg-emerald-50 p-4 text-sm text-emerald-800">
			The workbook details, date mappings, and learner mappings have no detected warnings.
		</div>
	{/if}

	<div class="flex flex-wrap justify-end gap-2">
		<button
			type="button"
			onclick={() => (exportDialogOpen = false)}
			class="control-ring h-10 rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-surface"
		>
			Cancel
		</button>
		<button
			type="button"
			onclick={onConfirmExport}
			disabled={exporting || !preview?.canExport}
			class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
		>
			<Save class="size-4" aria-hidden="true" />
			{exporting ? 'Exporting...' : 'Export Workbook'}
		</button>
	</div>
</Dialog>

<Dialog
	open={exportLoadingOpen}
	title="Exporting SF2 Workbook"
	description="Saving the reviewed workbook and opening the exported file."
	maxWidth="lg"
	showCloseButton={false}
>
	<TaskProgress
		active={exportLoadingOpen}
		title="Exporting SF2 workbook"
		description="Writing attendance marks, copying the workbook, and opening the generated file."
		simple
	/>
</Dialog>

<FeedbackToast message={toastMessage} ok={toastOk} onClose={() => (toastMessage = null)} />
