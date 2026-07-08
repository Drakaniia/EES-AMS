<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import {
		sf2ValidationDuplicateLabel,
		sf2ValidationLearnerLabel,
		sf2ValidationStudentLabel
	} from '$lib/features/settings/sf2-validation';
	import type { Sf2ImportValidation } from '$lib/features/settings/native';

	let {
		open = $bindable(false),
		validation = $bindable<Sf2ImportValidation | null>(null),
		importing = $bindable(false),
		detailsOpen = $bindable(false),
		onproceed,
		oncancel,
		ondownloadreport
	}: {
		open?: boolean;
		validation?: Sf2ImportValidation | null;
		importing?: boolean;
		detailsOpen?: boolean;
		onproceed?: () => void;
		oncancel?: () => void;
		ondownloadreport?: () => void;
	} = $props();
</script>

<Dialog
	{open}
	title="Warning: Student List Mismatch Detected"
	description="The imported SF2 does not match the current student records."
	maxWidth="xl"
	showCloseButton={!importing}
	onClose={() => oncancel?.()}
>
	{#if validation}
		<div class="space-y-5">
			<div class="rounded-md border border-amber-200 bg-amber-50 p-4 text-sm text-amber-950">
				<p>
					The import is paused until these discrepancies are reviewed and explicitly acknowledged.
				</p>
			</div>

			<div class="grid gap-3 sm:grid-cols-3">
				<div class="rounded-md border border-border bg-surface p-4">
					<div class="label-mono">Current Missing in SF2</div>
					<div class="mt-2 text-2xl font-semibold">{validation.missingFromSf2.length}</div>
				</div>
				<div class="rounded-md border border-border bg-surface p-4">
					<div class="label-mono">SF2 Missing Current</div>
					<div class="mt-2 text-2xl font-semibold">
						{validation.missingFromCurrent.length}
					</div>
				</div>
				<div class="rounded-md border border-border bg-surface p-4">
					<div class="label-mono">Potential Mismatches</div>
					<div class="mt-2 text-2xl font-semibold">
						{validation.possibleNameMismatches.length}
					</div>
				</div>
			</div>

			<div class="rounded-md border border-border p-4 text-sm">
				<div class="grid gap-3 sm:grid-cols-2">
					<div>
						<div class="label-mono">Matched Class</div>
						<div class="mt-1 font-medium">{validation.className}</div>
					</div>
					<div>
						<div class="label-mono">Roster Counts</div>
						<div class="mt-1 font-medium">
							{validation.currentStudentCount} current / {validation.sf2LearnerCount} SF2
						</div>
					</div>
				</div>
				<div class="mt-3 text-xs break-all text-muted-foreground">{validation.sourcePath}</div>
			</div>

			{#if detailsOpen}
				<div class="max-h-[45vh] space-y-4 overflow-y-auto pr-1">
					<section class="rounded-md border border-border p-4">
						<h4 class="text-sm font-semibold">
							Students Found in Current Records but Missing in SF2
						</h4>
						{#if validation.missingFromSf2.length > 0}
							<ul class="mt-3 space-y-1 text-sm text-muted-foreground">
								{#each validation.missingFromSf2 as student (student.studentId)}
									<li>{sf2ValidationStudentLabel(student)}</li>
								{/each}
							</ul>
						{:else}
							<p class="mt-3 text-sm text-muted-foreground">No records in this category.</p>
						{/if}
					</section>

					<section class="rounded-md border border-border p-4">
						<h4 class="text-sm font-semibold">
							Students Found in SF2 but Missing in Current Records
						</h4>
						{#if validation.missingFromCurrent.length > 0}
							<ul class="mt-3 space-y-1 text-sm text-muted-foreground">
								{#each validation.missingFromCurrent as learner (`${learner.rowIndex}-${learner.normalizedName}`)}
									<li>{sf2ValidationLearnerLabel(learner)}</li>
								{/each}
							</ul>
						{:else}
							<p class="mt-3 text-sm text-muted-foreground">No records in this category.</p>
						{/if}
					</section>

					<section class="rounded-md border border-border p-4">
						<h4 class="text-sm font-semibold">Potential Name Mismatches</h4>
						{#if validation.possibleNameMismatches.length > 0}
							<ul class="mt-3 space-y-2 text-sm text-muted-foreground">
								{#each validation.possibleNameMismatches as mismatch (`${mismatch.currentStudent.studentId}-${mismatch.sf2Learner.rowIndex}`)}
									<li>
										<span class="font-medium text-foreground">{mismatch.currentStudent.name}</span>
										<span> -> </span>
										<span class="font-medium text-foreground">{mismatch.sf2Learner.name}</span>
										<span class="block text-xs">{mismatch.reason}</span>
									</li>
								{/each}
							</ul>
						{:else}
							<p class="mt-3 text-sm text-muted-foreground">
								No potential name mismatches detected.
							</p>
						{/if}
					</section>

					<section class="rounded-md border border-border p-4">
						<h4 class="text-sm font-semibold">Additional Validation Checks</h4>
						<div class="mt-3 grid gap-4 md:grid-cols-3">
							<div>
								<div class="label-mono">Duplicate Current Records</div>
								{#if validation.duplicateCurrentStudents.length > 0}
									<ul class="mt-2 space-y-1 text-sm text-muted-foreground">
										{#each validation.duplicateCurrentStudents as duplicate (duplicate.normalizedName)}
											<li>{sf2ValidationDuplicateLabel(duplicate)}</li>
										{/each}
									</ul>
								{:else}
									<p class="mt-2 text-sm text-muted-foreground">None</p>
								{/if}
							</div>
							<div>
								<div class="label-mono">Duplicate SF2 Entries</div>
								{#if validation.duplicateSf2Learners.length > 0}
									<ul class="mt-2 space-y-1 text-sm text-muted-foreground">
										{#each validation.duplicateSf2Learners as duplicate (duplicate.normalizedName)}
											<li>{sf2ValidationDuplicateLabel(duplicate)}</li>
										{/each}
									</ul>
								{:else}
									<p class="mt-2 text-sm text-muted-foreground">None</p>
								{/if}
							</div>
							<div>
								<div class="label-mono">Missing Learner Information</div>
								{#if validation.missingLearnerInfo.length > 0}
									<ul class="mt-2 space-y-1 text-sm text-muted-foreground">
										{#each validation.missingLearnerInfo as learner (learner.rowIndex)}
											<li>{sf2ValidationLearnerLabel(learner)}</li>
										{/each}
									</ul>
								{:else}
									<p class="mt-2 text-sm text-muted-foreground">None</p>
								{/if}
							</div>
						</div>
					</section>
				</div>
			{/if}

			<div class="flex flex-wrap justify-end gap-2 pt-2">
				<button
					type="button"
					onclick={() => (detailsOpen = !detailsOpen)}
					disabled={importing}
					class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					Review Differences
				</button>
				<button
					type="button"
					onclick={ondownloadreport}
					disabled={importing}
					class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					Download Validation Report
				</button>
				<button
					type="button"
					onclick={oncancel}
					disabled={importing}
					class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
				>
					Cancel Import
				</button>
				<button
					type="button"
					onclick={onproceed}
					disabled={importing}
					class="inline-flex items-center justify-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
				>
					{#if importing}
						<Spinner />
					{/if}
					{importing ? 'Importing...' : 'Proceed Anyway (Authorized Users Only)'}
				</button>
			</div>
		</div>
	{/if}
</Dialog>
