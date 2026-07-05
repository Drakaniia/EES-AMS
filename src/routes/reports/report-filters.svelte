<script lang="ts">
	import {
		FileCheck2,
		UserX,
		CircleAlert,
		Settings2,
	} from 'lucide-svelte';
	import { SF2_SCHOOL_MONTHS } from '$lib/features/settings/sf2-workbook';
	import type { Sf2ExportPreview, Sf2WorkbookSettings } from '$lib/db-rust';
	import type { Class } from '$lib/db-rust';
	import { headerReviewValue, headerReviewMonthValue } from './report-state.svelte';

	let {
		preview,
		selectedClass,
		issueCount,
		warningCount,
		fullReview,
		fullReviewHeaderVisible,
		workbookSettings,
		savingDetails,
		draftSchoolId,
		draftSchoolName,
		draftSchoolYear,
		draftReportMonth,
		draftGradeLevel,
		draftSection,
		draftAdviserName,
		draftSchoolHeadName,
		onSaveWorkbookDetails,
		onReportMonthChange,
		onDraftChange,
	}: {
		preview: Sf2ExportPreview | null;
		selectedClass: Class | undefined;
		issueCount: number;
		warningCount: number;
		fullReview: boolean;
		fullReviewHeaderVisible: boolean;
		workbookSettings: Sf2WorkbookSettings | null;
		savingDetails: boolean;
		draftSchoolId: string;
		draftSchoolName: string;
		draftSchoolYear: string;
		draftReportMonth: string;
		draftGradeLevel: string;
		draftSection: string;
		draftAdviserName: string;
		draftSchoolHeadName: string;
		onSaveWorkbookDetails: (successMessage?: string | null) => Promise<boolean>;
		onReportMonthChange: () => void;
		onDraftChange: (field: string, value: string) => void;
	} = $props();
</script>

{#if fullReview && fullReviewHeaderVisible && preview?.template}
	<div class="border border-border bg-card p-5 shadow-sm rounded-xl">
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div>
				<div class="label-mono text-primary">SF2 workbook details</div>
				<h2 class="mt-1 text-xl font-semibold">
					{draftSchoolName || preview.template.schoolName || 'Name of School'}
				</h2>
			</div>
		</div>
		<dl class="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
			{@render headerReviewField('School ID', headerReviewValue(draftSchoolId, preview.template.schoolId, workbookSettings))}
			{@render headerReviewField('School Year', headerReviewValue(draftSchoolYear, preview.template.schoolYear, workbookSettings))}
			{@render headerReviewField('Report Month', headerReviewMonthValue(draftReportMonth, preview.template.reportMonth, workbookSettings))}
			{@render headerReviewField('Grade Level', headerReviewValue(draftGradeLevel, preview.template.gradeLevel, workbookSettings))}
			<div class="md:col-span-2">
				{@render headerReviewField('Name of School', headerReviewValue(draftSchoolName, preview.template.schoolName, workbookSettings))}
			</div>
			{@render headerReviewField('Section', headerReviewValue(draftSection, preview.template.section, workbookSettings))}
			{@render headerReviewField('Adviser / LIS Name', headerReviewValue(draftAdviserName, preview.template.adviserName, workbookSettings))}
			<div class="md:col-span-2">
				{@render headerReviewField('School Head Name', headerReviewValue(draftSchoolHeadName, preview.template.schoolHeadName, workbookSettings))}
			</div>
		</dl>
	</div>
{/if}

{#if !fullReview && preview?.template}
	<div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
		{@render summaryTile(
			'Workbook',
			preview.className || selectedClass?.name || 'Linked class',
			preview.canExport ? 'Ready for confirmation' : 'Needs attention',
			FileCheck2,
			preview.canExport ? 'ready' : 'warning'
		)}
		{@render summaryTile(
			'Absences',
			String(preview.absenceCount),
			`${preview.presentCount} present marks recorded`,
			UserX,
			preview.absenceCount > 0 ? 'alert' : 'ready'
		)}
		{@render summaryTile(
			'Checks',
			`${issueCount}/${warningCount}`,
			'Issues / warnings',
			CircleAlert,
			issueCount > 0 ? 'alert' : warningCount > 0 ? 'warning' : 'ready'
		)}
	</div>

	<div class="border border-border bg-card p-5 shadow-sm rounded-2xl">
		<div class="flex flex-wrap items-start justify-between gap-3">
			<div>
				<div class="label-mono text-primary">SF2 workbook details</div>
				<h2 class="mt-1 text-xl font-semibold">
					{draftSchoolName || preview.template.schoolName || 'Name of School'}
				</h2>
				<p class="mt-1 text-sm text-muted-foreground">
					These fields are written into the SF2 workbook before export.
				</p>
			</div>
			<button
				type="button"
				onclick={() => onSaveWorkbookDetails()}
				disabled={!workbookSettings || savingDetails}
				class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
			>
				<Settings2 class="size-4" aria-hidden="true" />
				{savingDetails ? 'Saving...' : 'Save Details'}
			</button>
		</div>

		<div class="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
			{@render textField('School ID', draftSchoolId, 'draftSchoolId')}
			{@render textField('School Year', draftSchoolYear, 'draftSchoolYear')}
			<label class="space-y-1.5">
				<span class="label-mono">Report Month</span>
				<select
					value={draftReportMonth}
					onchange={(e) => {
						onDraftChange('draftReportMonth', (e.currentTarget as HTMLSelectElement).value);
						onReportMonthChange();
					}}
					disabled={!workbookSettings || savingDetails}
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none disabled:opacity-60"
				>
					<option value="">Select month</option>
					{#each SF2_SCHOOL_MONTHS as month (month.value)}
						<option value={month.value}>{month.label}</option>
					{/each}
				</select>
			</label>
			{@render textField('Grade Level', draftGradeLevel, 'draftGradeLevel')}
			<div class="md:col-span-2">
				{@render textField('Name of School', draftSchoolName, 'draftSchoolName')}
			</div>
			{@render textField('Section', draftSection, 'draftSection')}
			{@render textField('Adviser / LIS Name', draftAdviserName, 'draftAdviserName')}
			<div class="md:col-span-2">
				{@render textField('School Head Name', draftSchoolHeadName, 'draftSchoolHeadName')}
			</div>
		</div>
	</div>
{/if}

{#snippet headerReviewField(label: string, value: string)}
	<div class="rounded-md border border-border bg-background px-3 py-2">
		<dt class="label-mono">{label}</dt>
		<dd class="mt-1 truncate text-sm font-semibold">{value}</dd>
	</div>
{/snippet}

{#snippet summaryTile(
	label: string,
	value: string,
	description: string,
	Icon: typeof FileCheck2,
	tone: 'neutral' | 'ready' | 'warning' | 'alert'
)}
	<div class="rounded-2xl border border-border bg-card p-4 shadow-sm">
		<div class="flex items-start justify-between gap-3">
			<div class="min-w-0">
				<div class="label-mono">{label}</div>
				<div class="mt-2 truncate text-2xl font-semibold">{value}</div>
				<div class="mt-1 truncate text-xs text-muted-foreground">{description}</div>
			</div>
			<div
				class="grid size-10 shrink-0 place-items-center rounded-md {tone === 'ready'
					? 'bg-emerald-50 text-emerald-700'
					: tone === 'warning'
						? 'bg-amber-50 text-amber-700'
						: tone === 'alert'
							? 'bg-red-50 text-red-700'
							: 'bg-surface text-muted-foreground'}"
			>
				<Icon class="size-5" aria-hidden="true" />
			</div>
		</div>
	</div>
{/snippet}

{#snippet textField(label: string, value: string, field: string)}
	<label class="space-y-1.5">
		<span class="label-mono">{label}</span>
		<input
			{value}
			oninput={(event) => {
				onDraftChange(field, (event.currentTarget as HTMLInputElement).value);
			}}
			disabled={!workbookSettings || savingDetails}
			class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none disabled:opacity-60"
		/>
	</label>
{/snippet}

{#snippet metaRow(label: string, value: string)}
	<div class="grid grid-cols-[112px_minmax(0,1fr)] gap-3">
		<dt class="text-xs font-medium text-muted-foreground">{label}</dt>
		<dd class="truncate font-medium">{value}</dd>
	</div>
{/snippet}
