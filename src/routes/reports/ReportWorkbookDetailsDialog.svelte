<script lang="ts">
	import { Save } from 'lucide-svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import { reportMonthLabel } from './report-state.svelte';
	import type { Sf2WorkbookSettings } from '$lib/db-rust';

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
	title="SF2 Workbook Details"
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
				class="h-10 w-full cursor-not-allowed rounded-md border border-border bg-muted/30 px-3 text-sm text-muted-foreground opacity-60"
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
			<p class="text-xs text-amber-600">You have unsaved changes.</p>
		{:else}
			<p class="text-xs text-muted-foreground">No changes detected.</p>
		{/if}
		<div class="flex gap-2">
			<button
				type="button"
				onclick={onClose}
				class="control-ring h-10 rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-surface"
			>
				Cancel
			</button>
			<button
				type="button"
				onclick={onSave}
				disabled={!hasModalDraftChanges || modalSaving || savingDetails}
				class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
			>
				<Save class="size-4" aria-hidden="true" />
				{modalSaving ? 'Saving...' : 'Save Details'}
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
			class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none disabled:opacity-60"
		/>
	</label>
{/snippet}
