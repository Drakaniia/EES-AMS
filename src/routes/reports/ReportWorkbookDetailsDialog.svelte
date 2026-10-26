<script lang="ts">
	import { Save } from 'lucide-svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import { reportMonthLabel } from './report-state.svelte';
	import type { Sf2WorkbookSettings } from '$lib/api';

	type Props = {
		open: boolean;
		workbookSettings: Sf2WorkbookSettings | null;
		draftSchoolId: string;
		draftSchoolName: string;
		draftSchoolYear: string;
		draftReportMonth: string;
		draftGradeLevel: string;
		draftSection: string;
		draftAdviserName: string;
		draftSchoolHeadName: string;
		hasModalDraftChanges: boolean;
		modalSaving: boolean;
		savingDetails: boolean;
		onClose?: () => void;
		onSave?: () => void;
		onDraftChange?: (field: string, value: string) => void;
	};

	let {
		open,
		workbookSettings,
		draftSchoolId,
		draftSchoolName,
		draftSchoolYear,
		draftReportMonth,
		draftGradeLevel,
		draftSection,
		draftAdviserName,
		draftSchoolHeadName,
		hasModalDraftChanges,
		modalSaving,
		savingDetails,
		onClose,
		onSave,
		onDraftChange
	}: Props = $props();
</script>

<Dialog
	{open}
	title="SF2 workbook details"
	description="Edit the header fields that are written into the SF2 workbook before export."
	maxWidth="2xl"
	{onClose}
>
	<div class="grid gap-4 md:grid-cols-2">
		{@render modalTextField('School ID', draftSchoolId, 'draftSchoolId')}
		{@render modalTextField('School Year', draftSchoolYear, 'draftSchoolYear')}
		<label class="space-y-1.5">
			<span class="label-mono">Report Month</span>
			<input
				value={reportMonthLabel(draftReportMonth || workbookSettings?.reportMonth || '')}
				disabled={true}
				class="field cursor-not-allowed bg-surface text-muted-foreground"
				title="Use the 'Switch month' button to change the report month"
			/>
		</label>
		{@render modalTextField('Grade Level', draftGradeLevel, 'draftGradeLevel')}
		<div class="md:col-span-2">
			{@render modalTextField('Name of School', draftSchoolName, 'draftSchoolName')}
		</div>
		{@render modalTextField('Section', draftSection, 'draftSection')}
		{@render modalTextField('Adviser / LIS Name', draftAdviserName, 'draftAdviserName')}
		<div class="md:col-span-2">
			{@render modalTextField('School Head Name', draftSchoolHeadName, 'draftSchoolHeadName')}
		</div>
	</div>

	<div class="flex flex-wrap items-center justify-between gap-3 pt-2">
		{#if hasModalDraftChanges}
			<p class="text-[11.5px] font-medium text-amber-700 dark:text-amber-400">Unsaved changes</p>
		{:else}
			<p class="text-xs text-muted-foreground">No changes detected.</p>
		{/if}
		<div class="flex gap-2">
			<button type="button" onclick={onClose} class="btn btn-secondary">Cancel</button>
			<button
				type="button"
				onclick={onSave}
				disabled={!hasModalDraftChanges || modalSaving || savingDetails}
				class="btn btn-primary"
			>
				<Save class="size-4" aria-hidden="true" strokeWidth={1.9} />
				{modalSaving ? 'Saving…' : 'Save details'}
			</button>
		</div>
	</div>
</Dialog>

{#snippet modalTextField(label: string, value: string, field: string)}
	<label class="space-y-1.5">
		<span class="label-mono">{label}</span>
		<input
			{value}
			oninput={(event) => {
				onDraftChange?.(field, (event.currentTarget as HTMLInputElement).value);
			}}
			disabled={!workbookSettings || modalSaving}
			class="field disabled:opacity-60"
		/>
	</label>
{/snippet}
