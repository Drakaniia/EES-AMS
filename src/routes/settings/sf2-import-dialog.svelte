<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import type { Sf2ImportValidation } from '$lib/features/sf2/validation';

	interface Props {
		open: boolean;
		validation: Sf2ImportValidation | null;
		busy: boolean;
		onProceed: () => void;
		onCancel: () => void;
	}

	let { open, validation, busy, onProceed, onCancel }: Props = $props();
</script>

<Dialog
	{open}
	title="Import workbook"
	description="Review what the workbook and the app disagree about before the workbook becomes the class record."
	maxWidth="lg"
	onClose={onCancel}
>
	{#if validation}
		<div class="space-y-3 text-sm">
			<p>
				{validation.currentStudentCount} students on record, {validation.sf2LearnerCount}
				learners in the workbook ({validation.className}).
			</p>
			{#if validation.missingFromSf2.length > 0}
				<p>
					{validation.missingFromSf2.length} missing from workbook:
					{validation.missingFromSf2
						.slice(0, 5)
						.map((s) => s.name)
						.join('; ')}{validation.missingFromSf2.length > 5 ? '…' : ''}
				</p>
			{/if}
			{#if validation.missingFromCurrent.length > 0}
				<p>
					{validation.missingFromCurrent.length} new in workbook:
					{validation.missingFromCurrent
						.slice(0, 5)
						.map((l) => l.name)
						.join('; ')}{validation.missingFromCurrent.length > 5 ? '…' : ''}
				</p>
			{/if}
			{#if validation.possibleNameMismatches.length > 0}
				<p>
					{validation.possibleNameMismatches.length} possible name mismatches — check the spelling in
					Excel before proceeding.
				</p>
			{/if}
			{#if validation.duplicateCurrentStudents.length > 0 || validation.duplicateSf2Learners.length > 0}
				<p>
					{validation.duplicateCurrentStudents.length + validation.duplicateSf2Learners.length} duplicate
					names — fix them before importing.
				</p>
			{/if}
			{#if !validation.hasDiscrepancies}
				<p>No disagreements. Importing adopts the workbook's roster as-is.</p>
			{/if}
			<div class="flex justify-end gap-3">
				<button
					type="button"
					onclick={onCancel}
					disabled={busy}
					class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					Cancel
				</button>
				<button
					type="button"
					onclick={onProceed}
					disabled={busy}
					class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if busy}
						<Spinner />
					{/if}
					{busy ? 'Importing…' : 'Import anyway'}
				</button>
			</div>
		</div>
	{/if}
</Dialog>
