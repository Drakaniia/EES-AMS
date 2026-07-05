<script lang="ts">
	import { settingsState } from './settings-state.svelte.ts';
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
				Import the official SF2 .xls form, or create a first-month working copy from the
				bundled template.
			</p>
		</div>
		<div class="flex flex-wrap gap-2">
			<button
				onclick={() => settingsState.openSf2TemplateDialog()}
				disabled={settingsState.sf2TemplateCreating || settingsState.sf2SettingsSaving}
				class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if settingsState.sf2TemplateCreating || settingsState.sf2SettingsSaving}
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
				{settingsState.sf2TemplateCreating
					? 'Creating...'
					: settingsState.sf2SettingsSaving
						? 'Saving...'
						: 'Create From Template'}
			</button>
			<button
				onclick={() => settingsState.onImportSf2()}
				disabled={settingsState.sf2Importing}
				class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
			>
				{#if settingsState.sf2Importing}
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
				{settingsState.sf2Importing ? 'Importing...' : 'Import SF2'}
			</button>
		</div>
	</div>

	{#if settingsState.sf2ImportSummary}
		<div class="space-y-4 border-t border-border pt-5">
			<div class="grid gap-3 sm:grid-cols-4">
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono">Class</div>
					<div class="mt-2 text-sm font-semibold">{settingsState.sf2ImportSummary.className}</div>
				</div>
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono">Learners</div>
					<div class="mt-2 text-2xl font-semibold">{settingsState.sf2ImportSummary.learnersFound}</div>
				</div>
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono">Created</div>
					<div class="mt-2 text-2xl font-semibold">
						{settingsState.sf2ImportSummary.studentsCreated}
					</div>
				</div>
				<div class="rounded-xl border border-border bg-surface p-4">
					<div class="label-mono">Dates</div>
					<div class="mt-2 text-2xl font-semibold">{settingsState.sf2ImportSummary.datesMapped}</div>
				</div>
			</div>
			<div class="flex justify-end">
				<button
					onclick={() => settingsState.startSf2Attendance()}
					class="rounded-pill bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
				>
					Start Attendance
				</button>
			</div>
		</div>
	{/if}
</section>

<Sf2ImportValidationDialog
	bind:open={settingsState.sf2ValidationDialogOpen}
	bind:validation={settingsState.sf2Validation}
	bind:importing={settingsState.sf2Importing}
	bind:detailsOpen={settingsState.sf2ValidationDetailsOpen}
	onproceed={() => settingsState.proceedWithSf2MismatchImport()}
	oncancel={() => settingsState.cancelSf2ValidationImport()}
	ondownloadreport={() => settingsState.downloadSf2ValidationReport()}
/>

<Sf2TemplateDialog
	bind:open={settingsState.sf2TemplateDialogOpen}
	bind:mode={settingsState.sf2TemplateDialogMode}
	bind:notice={settingsState.sf2TemplateDialogNotice}
	bind:creating={settingsState.sf2TemplateCreating}
	bind:saving={settingsState.sf2SettingsSaving}
	bind:classId={settingsState.sf2TemplateClassId}
	bind:schoolId={settingsState.sf2DraftSchoolId}
	bind:schoolName={settingsState.sf2DraftSchoolName}
	bind:schoolYear={settingsState.sf2DraftSchoolYear}
	bind:reportMonth={settingsState.sf2DraftReportMonth}
	bind:gradeLevel={settingsState.sf2DraftGradeLevel}
	bind:section={settingsState.sf2DraftSection}
	bind:adviserName={settingsState.sf2DraftAdviserName}
	bind:schoolHeadName={settingsState.sf2DraftSchoolHeadName}
	bind:firstSchoolDay={settingsState.sf2DraftFirstSchoolDay}
	onselectReportMonth={(monthValue) => {
		const schoolYear = settingsState.sf2DraftSchoolYear.trim() || defaultSf2SchoolYear();
		settingsState.sf2DraftReportMonth = monthValue;
		settingsState.sf2DraftSchoolYear = schoolYear;
		settingsState.sf2DraftFirstSchoolDay = defaultSf2FirstSchoolDay(monthValue, schoolYear);
	}}
	onupdateSchoolYear={(value) => {
		settingsState.sf2DraftSchoolYear = value;
		settingsState.sf2DraftFirstSchoolDay = normalizedSf2FirstSchoolDay(
			settingsState.sf2DraftReportMonth,
			value,
			settingsState.sf2DraftFirstSchoolDay
		);
	}}
	onselectFirstSchoolDay={(day) => {
		if (day === null) return;
		if (!isSf2SchoolDay(settingsState.sf2DraftReportMonth, settingsState.sf2DraftSchoolYear, day)) return;
		settingsState.sf2DraftFirstSchoolDay = day;
	}}
	onsubmit={(e) => settingsState.onCreateSf2FromTemplate(e)}
	onclose={(force) => settingsState.closeSf2TemplateDialog(force)}
/>
