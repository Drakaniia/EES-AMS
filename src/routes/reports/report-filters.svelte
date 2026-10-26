<script lang="ts">
	import type { Sf2ExportPreview, Sf2WorkbookSettings } from '$lib/api';
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
	<div class="rounded-xl border border-border bg-card p-4">
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div>
				<p class="label-mono">SF2 workbook details</p>
				<h2 class="mt-1 text-[16px] font-semibold tracking-[-0.015em]">
					{draftSchoolName || preview.template.schoolName || 'Name of School'}
				</h2>
			</div>
		</div>
		<dl class="mt-4 grid gap-2 md:grid-cols-2 xl:grid-cols-4">
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
	<div class="rounded-md bg-surface/60 px-3 py-2">
		<dt class="text-[10.5px] font-medium text-muted-foreground">{label}</dt>
		<dd class="mt-0.5 truncate text-[12.5px] font-semibold">{value}</dd>
	</div>
{/snippet}
