<script lang="ts">
	import type { Sf2ExportPreview, Sf2WorkbookSettings } from '$lib/db-rust';
	import { headerReviewValue, headerReviewMonthValue } from './report-state.svelte';

	let {
		preview,
		fullReview,
		fullReviewHeaderVisible,
		workbookSettings,
		draftSchoolId,
		draftSchoolName,
		draftSchoolYear,
		draftReportMonth,
		draftGradeLevel,
		draftSection,
		draftAdviserName,
		draftSchoolHeadName
	}: {
		preview: Sf2ExportPreview | null;
		fullReview: boolean;
		fullReviewHeaderVisible: boolean;
		workbookSettings: Sf2WorkbookSettings | null;
		draftSchoolId: string;
		draftSchoolName: string;
		draftSchoolYear: string;
		draftReportMonth: string;
		draftGradeLevel: string;
		draftSection: string;
		draftAdviserName: string;
		draftSchoolHeadName: string;
	} = $props();
</script>

{#if fullReview && fullReviewHeaderVisible && preview?.template}
	<div class="rounded-xl border border-border bg-card p-5 shadow-sm">
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div>
				<div class="label-mono text-primary">SF2 workbook details</div>
				<h2 class="mt-1 text-xl font-semibold">
					{draftSchoolName || preview.template.schoolName || 'Name of School'}
				</h2>
			</div>
		</div>
		<dl class="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
			{@render headerReviewField(
				'School ID',
				headerReviewValue(draftSchoolId, preview.template.schoolId, workbookSettings)
			)}
			{@render headerReviewField(
				'School Year',
				headerReviewValue(draftSchoolYear, preview.template.schoolYear, workbookSettings)
			)}
			{@render headerReviewField(
				'Report Month',
				headerReviewMonthValue(draftReportMonth, preview.template.reportMonth, workbookSettings)
			)}
			{@render headerReviewField(
				'Grade Level',
				headerReviewValue(draftGradeLevel, preview.template.gradeLevel, workbookSettings)
			)}
			<div class="md:col-span-2">
				{@render headerReviewField(
					'Name of School',
					headerReviewValue(draftSchoolName, preview.template.schoolName, workbookSettings)
				)}
			</div>
			{@render headerReviewField(
				'Section',
				headerReviewValue(draftSection, preview.template.section, workbookSettings)
			)}
			{@render headerReviewField(
				'Adviser / LIS Name',
				headerReviewValue(draftAdviserName, preview.template.adviserName, workbookSettings)
			)}
			<div class="md:col-span-2">
				{@render headerReviewField(
					'School Head Name',
					headerReviewValue(draftSchoolHeadName, preview.template.schoolHeadName, workbookSettings)
				)}
			</div>
		</dl>
	</div>
{/if}

{#snippet headerReviewField(label: string, value: string)}
	<div class="rounded-md border border-border bg-background px-3 py-2">
		<dt class="label-mono">{label}</dt>
		<dd class="mt-1 truncate text-sm font-semibold">{value}</dd>
	</div>
{/snippet}
