<script lang="ts">
	import { Calendar, ChevronDown, ExternalLink, Pencil, Plus, UserX } from 'lucide-svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import {
		reportMonthLabel,
		formatImportedAt,
		formatDate,
		workbookStatusLine
	} from './report-state.svelte';
	import type { Class, Sf2ExportPreview, Sf2WorkbookSettings } from '$lib/api';

	type Props = {
		preview: Sf2ExportPreview | null;
		/**
		 * The grid is re-reading. This is the *only* loading state the Reports page
		 * has: the sidebar, the month picker and the class selector all stay
		 * interactive while it is true (spec §7.4, acceptance #10).
		 */
		gridPending: boolean;
		classes: Class[];
		selectedClassId: string;
		reportMonth: string;
		creatingMonth: boolean;
		/**
		 * The guard's own comparison for the month on screen: how many X the
		 * workbook file holds, and `null` when it has never been counted. Null is
		 * not zero - an unmeasured file is not an empty one.
		 */
		workbookXCount: number | null;
		/**
		 * When that count was taken, in Unix seconds, or `null` for a month whose
		 * file has never been counted. The startup self-heal writes it; the line
		 * below only reports it.
		 */
		workbookScannedAt: number | null;
		/** Today's month has school days and no file: offer the E1 one-click create. */
		needsMonthCreate: boolean;
		createMonthName: string;
		selectedClass: Class | null | undefined;
		draftSchoolId: string;
		draftSchoolYear: string;
		draftReportMonth: string;
		draftGradeLevel: string;
		draftSection: string;
		draftAdviserName: string;
		draftSchoolHeadName: string;
		sf2OpenStatus: string;
		workbookSettings: Sf2WorkbookSettings | null;
		savingDetails: boolean;
		activeClassId: string;
		/** Calendar year the month belongs to, for the "October 2025" label. */
		reportYear?: number;
		onOpenSf2?: () => void;
		onClassSelect?: (classId: string) => void;
		onCreateMonth?: () => void;
		onEditDetails?: () => void;
		onSwitchMonth?: () => void;
	};

	let {
		preview,
		gridPending = false,
		classes = [],
		selectedClassId,
		reportMonth,
		creatingMonth = false,
		workbookXCount = null,
		workbookScannedAt = null,
		needsMonthCreate = false,
		createMonthName = '',
		selectedClass,
		draftSchoolId,
		draftSchoolYear,
		draftReportMonth,
		draftGradeLevel,
		draftSection,
		draftAdviserName,
		draftSchoolHeadName,
		sf2OpenStatus,
		workbookSettings,
		savingDetails,
		activeClassId,
		reportYear,
		onOpenSf2,
		onClassSelect,
		onCreateMonth,
		onEditDetails,
		onSwitchMonth
	}: Props = $props();

	/**
	 * The guard's own comparison, surfaced as information rather than as a
	 * recovery panel (spec §12.2).
	 *
	 * The amber "no X marks in the app for this month" panel is gone, and so is
	 * the button that used to sit above it. Recovering the workbook's marks is the
	 * startup self-heal's job now, unattended and additively, so there is nothing
	 * left for a teacher to do about it - and a manual button for a repair that
	 * has already happened automatically is worse than a sentence. What is left is
	 * the evidence, read-only: when the file was last counted, how many X it
	 * holds, and how many the app has. If those two ever disagree, §9.1's guard
	 * refuses every write until they agree, which is the point of showing the
	 * line at all.
	 */
	const markCounts = $derived(
		workbookStatusLine({
			scannedAt: workbookScannedAt,
			workbookXCount,
			appXCount: preview?.absenceCount ?? 0
		})
	);

	// The grid shows nothing when the database has no absences for this report
	// month — which is what a rebuilt database looks like. Said plainly, with no
	// recovery button: the startup self-heal has already looked in the workbook
	// for it, and §12.2's status line above says what it found.
	const showNoAbsences = $derived(
		!!preview?.template && !gridPending && (preview?.absentList.length ?? 0) === 0
	);
</script>

<aside
	class="min-h-0 overflow-y-auto border-t border-border bg-background xl:border-t-0 xl:border-l"
>
	<section class="border-b border-border px-4 py-4">
		<p class="label-mono mb-2.5">Class and month</p>

		<label class="block text-[11.5px] font-medium text-muted-foreground" for="reports-class-select">
			Class
		</label>
		<select
			id="reports-class-select"
			value={selectedClassId}
			onchange={(event) => onClassSelect?.(event.currentTarget.value)}
			disabled={classes.length === 0}
			class="field mt-1 font-medium"
		>
			{#each classes as item (item.id)}
				<option value={item.id}>{item.name}</option>
			{/each}
		</select>

		<label
			class="mt-3 block text-[11.5px] font-medium text-muted-foreground"
			for="reports-month-switch"
		>
			Report month
		</label>
		<button
			id="reports-month-switch"
			type="button"
			onclick={onSwitchMonth}
			disabled={!workbookSettings || !activeClassId}
			class="control-ring mt-1 flex w-full items-center gap-2.5 rounded-md border border-border bg-card px-3 py-2 text-left transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
			title="Switch SF2 report month"
		>
			<Calendar
				class="size-3.5 shrink-0 text-muted-foreground"
				aria-hidden="true"
				strokeWidth={1.9}
			/>
			<span class="min-w-0 flex-1">
				<span class="block truncate text-[13px] font-semibold">
					{reportMonthLabel(reportMonth)}{#if reportYear}
						{reportYear}{/if}
				</span>
				<span class="block text-[11px] text-muted-foreground">Change month</span>
			</span>
			<ChevronDown class="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
		</button>

		<!--
			Edge case E1: today's month has school days and no file, so the app
			fell back to the last-used month. The offer is here, in the sidebar,
			rather than on a toast that is gone before it has been read. Never shown
			for a month with no school days - the backend refuses those.
		-->
		{#if needsMonthCreate && createMonthName}
			<div
				class="mt-3 rounded-lg border border-amber-500/30 bg-amber-50/70 p-3 text-[11.5px] leading-5 text-amber-900 dark:border-amber-400/25 dark:bg-amber-950/25 dark:text-amber-200"
			>
				<p>
					Showing {reportMonthLabel(reportMonth)}. Create {reportMonthLabel(createMonthName)} to switch.
				</p>
				<button
					type="button"
					onclick={onCreateMonth}
					disabled={creatingMonth}
					class="btn btn-secondary btn-sm mt-2"
				>
					{#if creatingMonth}
						<Spinner />
					{:else}
						<Plus class="size-3.5" aria-hidden="true" />
					{/if}
					{creatingMonth ? 'Creating…' : `Create ${reportMonthLabel(createMonthName)}`}
				</button>
			</div>
		{/if}
	</section>

	<section class="border-b border-border px-4 py-4">
		<button
			type="button"
			onclick={onOpenSf2}
			disabled={!preview?.template || sf2OpenStatus === 'syncing' || !activeClassId}
			class="btn btn-primary w-full"
		>
			{#if sf2OpenStatus === 'syncing'}
				<Spinner />
				Opening…
			{:else}
				<ExternalLink class="size-4" aria-hidden="true" strokeWidth={1.9} />
				Open SF2 in Excel
			{/if}
		</button>
	</section>

	<section class="border-b border-border px-4 py-4">
		<div class="flex items-center justify-between gap-2">
			<p class="label-mono">Workbook identity</p>
			<button
				type="button"
				onclick={onEditDetails}
				disabled={!workbookSettings || savingDetails || !activeClassId}
				class="btn btn-ghost btn-sm"
				title="Edit workbook details"
			>
				<Pencil class="size-3" aria-hidden="true" />
				Edit
			</button>
		</div>
		<dl class="mt-2 divide-y divide-border text-[12.5px]">
			{@render metaRow('Class', preview?.className || selectedClass?.name || 'Unlinked')}
			{@render metaRow('School ID', draftSchoolId || preview?.template?.schoolId || 'Blank')}
			{@render metaRow('School Year', draftSchoolYear || preview?.template?.schoolYear || 'Blank')}
			{@render metaRow(
				'Report Month',
				reportMonthLabel(draftReportMonth || preview?.template?.reportMonth || '')
			)}
			{@render metaRow('Grade Level', draftGradeLevel || preview?.template?.gradeLevel || 'Blank')}
			{@render metaRow('Section', draftSection || preview?.template?.section || 'Blank')}
			{@render metaRow('Adviser', draftAdviserName || preview?.template?.adviserName || 'Blank')}
			{@render metaRow(
				'School Head',
				draftSchoolHeadName || preview?.template?.schoolHeadName || 'Blank'
			)}
			{@render metaRow('Imported', formatImportedAt(preview?.template?.importedAt))}
		</dl>

		{#if gridPending}
			<div class="mt-3 flex items-center gap-2 text-[11.5px] text-muted-foreground">
				<span class="size-1.5 rounded-full bg-muted-foreground/45" aria-hidden="true"></span>
				Reading the month…
			</div>
		{:else if preview?.canExport !== undefined}
			<div class="mt-3 flex items-center gap-2 text-[11.5px]">
				<span
					class="size-1.5 shrink-0 rounded-full {preview.canExport
						? 'bg-emerald-500'
						: 'bg-amber-500'}"
					aria-hidden="true"
				></span>
				<span class="text-muted-foreground">
					{preview.canExport ? 'Ready for export' : 'Needs attention'}
				</span>
			</div>
		{/if}

		<!-- Passive, in place of the old amber recovery panel (spec §12.2). -->
		{#if !gridPending && preview?.template}
			<p class="mt-2.5 text-[11px] leading-[1.55] text-muted-foreground">
				{markCounts.text}{#if markCounts.agrees === false}
					<span class="mt-1 block font-medium text-amber-700 dark:text-amber-300">
						The workbook and the app disagree. Nothing will be cleared until they agree.
					</span>{/if}
			</p>
		{/if}
	</section>

	<section class="px-4 py-4">
		<div class="flex items-start justify-between gap-3">
			<div>
				<p class="label-mono">Absent list</p>
				{#if gridPending}
					<div class="skeleton mt-1.5 h-5 w-24 rounded"></div>
				{:else}
					<h2 class="mt-1 text-[15px] font-semibold tracking-[-0.01em]">
						{preview?.absentList.length ?? 0}
						{(preview?.absentList.length ?? 0) === 1 ? 'entry' : 'entries'}
					</h2>
				{/if}
			</div>
			<UserX class="size-4 text-muted-foreground" aria-hidden="true" strokeWidth={1.9} />
		</div>

		{#if (preview?.absentList.length ?? 0) > 0}
			<ul class="mt-2 max-h-80 divide-y divide-border overflow-auto pr-0.5">
				{#each preview!.absentList as absence (`${absence.studentId}-${absence.date}`)}
					<li class="py-2">
						<div class="text-[12.5px] font-medium">{absence.studentName}</div>
						<div
							class="mt-0.5 flex items-center justify-between gap-3 text-[11px] text-muted-foreground"
						>
							<span>{formatDate(absence.date)}</span>
							<span class="tabular-nums">Row {absence.rowIndex}</span>
						</div>
					</li>
				{/each}
			</ul>
		{:else if showNoAbsences}
			<p class="mt-2 text-[12px] leading-5 text-muted-foreground">
				No absences are marked for this report month.
			</p>
		{/if}
	</section>
</aside>

{#snippet metaRow(label: string, value: string)}
	<div class="flex items-center justify-between gap-3 py-1.5">
		<dt class="text-muted-foreground">{label}</dt>
		<dd class="min-w-0 truncate text-right font-medium">{value}</dd>
	</div>
{/snippet}
