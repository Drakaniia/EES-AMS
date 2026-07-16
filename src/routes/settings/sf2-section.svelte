<script lang="ts">
	import { sf2State, classState } from './settings-state.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import Sf2ImportValidationDialog from './sf2-import-validation-dialog.svelte';
	import Sf2TemplateDialog from './sf2-template-dialog.svelte';
	import {
		defaultSf2FirstSchoolDay,
		defaultSf2SchoolYear,
		isSf2SchoolDay,
		normalizedSf2FirstSchoolDay
	} from '$lib/features/settings/sf2-workbook';
</script>

<section class="order-3 space-y-5 rounded-2xl border border-border bg-card p-6">
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div>
			<h3 class="text-lg font-medium">SF2 Workbook</h3>
			<p class="mt-1 text-sm text-muted-foreground">
				Import the official SF2 .xls form, or create a first-month working copy from the bundled
				template.
			</p>
		</div>
		<div class="flex flex-wrap gap-2">
			<button
				onclick={() => sf2State.openSf2TemplateDialog(classState.classes)}
				disabled={sf2State.sf2TemplateCreating || sf2State.sf2SettingsSaving}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if sf2State.sf2TemplateCreating || sf2State.sf2SettingsSaving}
					<Spinner />
				{:else}
					<svg
						class="size-4"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
						<polyline points="14 2 14 8 20 8" />
						<path d="M12 11v6" />
						<path d="M9 14h6" />
					</svg>
				{/if}
				{sf2State.sf2TemplateCreating
					? 'Creating...'
					: sf2State.sf2SettingsSaving
						? 'Saving...'
						: 'Create From Template'}
			</button>
			<button
				onclick={() => sf2State.onImportSf2()}
				disabled={sf2State.sf2Importing}
				class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if sf2State.sf2Importing}
					<Spinner />
				{:else}
					<svg
						class="size-4"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
						<polyline points="14 2 14 8 20 8" />
						<path d="M12 18v-6" />
						<path d="m9 15 3 3 3-3" />
					</svg>
				{/if}
				{sf2State.sf2Importing ? 'Importing...' : 'Import SF2'}
			</button>
		</div>
	</div>

	{#if sf2State.sf2ImportSummary}
		<div class="space-y-4 border-t border-border pt-5">
			<div class="grid gap-3 sm:grid-cols-4">
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono">Class</div>
					<div class="mt-2 text-sm font-semibold">{sf2State.sf2ImportSummary.className}</div>
				</div>
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono">Learners</div>
					<div class="mt-2 text-2xl font-semibold">{sf2State.sf2ImportSummary.learnersFound}</div>
				</div>
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono">Created</div>
					<div class="mt-2 text-2xl font-semibold">
						{sf2State.sf2ImportSummary.studentsCreated}
					</div>
				</div>
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono">Dates</div>
					<div class="mt-2 text-2xl font-semibold">{sf2State.sf2ImportSummary.datesMapped}</div>
				</div>
			</div>
			<div class="flex justify-end">
				<button
					onclick={() => sf2State.startSf2Attendance()}
					class="rounded-pill bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
				>
					Start Attendance
				</button>
			</div>
		</div>
	{/if}
</section>

{#if sf2State.sf2ProgressVisible}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		role="dialog"
		aria-modal="true"
		aria-label="SF2 {sf2State.sf2ProgressTask} in progress"
		class="fixed inset-0 z-[70] flex items-center justify-center bg-background/40 backdrop-blur-[2px]"
		tabindex="-1"
	>
		<div
			class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
			role="status"
			aria-live="polite"
		>
			<!-- Current message -->
			<div class="space-y-1">
				<p class="text-sm font-semibold text-foreground transition-all duration-300">
					{sf2State.sf2ProgressDisplayMessage ||
						(sf2State.sf2ProgressTask === 'import'
							? 'Importing SF2 workbook…'
							: 'Creating SF2 workbook…')}
				</p>
			</div>

			<!-- Determinate progress bar -->
			{#if sf2State.sf2ProgressTotal > 0}
				<div class="w-full space-y-2">
					<div
						class="h-3 w-full overflow-hidden rounded-pill border border-primary/20 bg-background"
						role="progressbar"
						aria-valuemin="0"
						aria-valuemax={sf2State.sf2ProgressTotal}
						aria-valuenow={sf2State.sf2ProgressCurrent}
						aria-valuetext={`{Math.round((sf2State.sf2ProgressCurrent / sf2State.sf2ProgressTotal) * 100)} percent`}
					>
						<div
							class="h-full rounded-pill bg-primary transition-all duration-400 ease-out"
							style="width: {sf2State.sf2ProgressTotal > 0
								? Math.round((sf2State.sf2ProgressCurrent / sf2State.sf2ProgressTotal) * 100)
								: 0}%"
						></div>
					</div>
					<div class="label-mono text-xs text-primary">
						Step {sf2State.sf2ProgressCurrent} of {sf2State.sf2ProgressTotal}
					</div>
				</div>
			{:else}
				<!-- Indeterminate progress when total is unknown -->
				<div class="w-full">
					<div
						class="h-3 w-full overflow-hidden rounded-pill border border-primary/20 bg-background"
						role="progressbar"
						aria-label="Loading"
					>
						<div
							class="h-full rounded-pill bg-primary indeterminate-progress"
						></div>
					</div>
				</div>
			{/if}

			{#if sf2State.sf2ProgressCurrent === sf2State.sf2ProgressTotal && sf2State.sf2ProgressTotal > 0}
				<p class="text-xs text-muted-foreground">Finalizing…</p>
			{/if}
		</div>
	</div>
{/if}

<style>
	.indeterminate-progress {
		animation: indeterminate-slide 2s ease-in-out infinite;
		width: 40%;
	}
	@keyframes indeterminate-slide {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(350%);
		}
	}
</style>

<Sf2ImportValidationDialog
	bind:open={sf2State.sf2ValidationDialogOpen}
	bind:validation={sf2State.sf2Validation}
	bind:importing={sf2State.sf2Importing}
	bind:detailsOpen={sf2State.sf2ValidationDetailsOpen}
	onproceed={() => sf2State.proceedWithSf2MismatchImport()}
	oncancel={() => sf2State.cancelSf2ValidationImport()}
	ondownloadreport={() => sf2State.downloadSf2ValidationReport()}
/>

<Sf2TemplateDialog
	bind:open={sf2State.sf2TemplateDialogOpen}
	bind:mode={sf2State.sf2TemplateDialogMode}
	bind:notice={sf2State.sf2TemplateDialogNotice}
	bind:creating={sf2State.sf2TemplateCreating}
	bind:saving={sf2State.sf2SettingsSaving}
	bind:schoolId={sf2State.sf2DraftSchoolId}
	bind:schoolName={sf2State.sf2DraftSchoolName}
	bind:schoolYear={sf2State.sf2DraftSchoolYear}
	bind:reportMonth={sf2State.sf2DraftReportMonth}
	bind:gradeLevel={sf2State.sf2DraftGradeLevel}
	bind:section={sf2State.sf2DraftSection}
	bind:adviserName={sf2State.sf2DraftAdviserName}
	bind:schoolHeadName={sf2State.sf2DraftSchoolHeadName}
	bind:firstSchoolDay={sf2State.sf2DraftFirstSchoolDay}
	onselectReportMonth={(monthValue) => {
		const schoolYear = sf2State.sf2DraftSchoolYear.trim() || defaultSf2SchoolYear();
		sf2State.sf2DraftReportMonth = monthValue;
		sf2State.sf2DraftSchoolYear = schoolYear;
		sf2State.sf2DraftFirstSchoolDay = defaultSf2FirstSchoolDay(monthValue, schoolYear);
	}}
	onupdateSchoolYear={(value) => {
		sf2State.sf2DraftSchoolYear = value;
		sf2State.sf2DraftFirstSchoolDay = normalizedSf2FirstSchoolDay(
			sf2State.sf2DraftReportMonth,
			value,
			sf2State.sf2DraftFirstSchoolDay
		);
	}}
	onselectFirstSchoolDay={(day) => {
		if (day === null) return;
		if (!isSf2SchoolDay(sf2State.sf2DraftReportMonth, sf2State.sf2DraftSchoolYear, day)) return;
		sf2State.sf2DraftFirstSchoolDay = day;
	}}
	onsubmit={(e) => sf2State.onCreateSf2FromTemplate(e)}
	onclose={(force) => sf2State.closeSf2TemplateDialog(force)}
/>
