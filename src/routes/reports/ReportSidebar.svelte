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

<aside class="min-h-0 overflow-y-auto border-t border-border bg-background xl:border-t-0 xl:border-l">
	<section class="border-b border-border px-5 py-5">
		<div class="label-mono mb-4 text-primary">Class &amp; month</div>
		<label class="block text-xs text-muted-foreground" for="reports-class-select">Class</label>
		<select
			id="reports-class-select"
			value={selectedClassId}
			onchange={(event) => onClassSelect?.(event.currentTarget.value)}
			disabled={classes.length === 0}
			class="control-ring mt-1 h-10 w-full rounded-md border border-border bg-background px-3 text-sm font-medium hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
		>
			{#each classes as item (item.id)}
				<option value={item.id}>{item.name}</option>
			{/each}
		</select>

			<label class="mt-4 block text-xs text-muted-foreground" for="reports-month-switch">
			Report month
		</label>
		<button
			id="reports-month-switch"
			type="button"
			onclick={onSwitchMonth}
			disabled={!workbookSettings || !activeClassId}
			class="control-ring mt-1 flex h-12 w-full items-center gap-3 rounded-md border border-border bg-background px-3 text-left transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
			title="Switch SF2 report month"
		>
			<Calendar class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
			<span class="min-w-0 flex-1">
				<span class="block truncate text-sm font-semibold">
					{reportMonthLabel(reportMonth)}{#if reportYear} {reportYear}{/if}
				</span>
				<span class="block text-xs text-muted-foreground">Tap to switch month</span>
			</span>
			<ChevronDown class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
		</button>

		<!--
			Edge case E1: today's month has school days and no file, so the app
			fell back to the last-used month. The offer is here, in the sidebar,
			rather than on a toast that is gone before it has been read. Never shown
			for a month with no school days - the backend refuses those.
		-->
		{#if needsMonthCreate && createMonthName}
			<div
				class="mt-4 rounded-md border border-amber-500/40 bg-amber-50/60 p-3 text-xs leading-5 text-amber-900"
			>
				<p>
					Showing {reportMonthLabel(reportMonth)}. Create {reportMonthLabel(createMonthName)} to switch.
				</p>
				<button
					type="button"
					onclick={onCreateMonth}
					disabled={creatingMonth}
					class="control-ring mt-2 inline-flex h-8 items-center gap-1.5 rounded-md border border-amber-500/50 bg-background px-2.5 text-xs font-semibold text-amber-900 transition-colors hover:bg-amber-100 disabled:cursor-not-allowed disabled:opacity-50"
				>
					{#if creatingMonth}
						<Spinner />
					{:else}
						<Plus class="size-3.5" aria-hidden="true" />
					{/if}
					{creatingMonth ? 'Creating...' : `Create ${reportMonthLabel(createMonthName)}`}
				</button>
			</div>
		{/if}
	</section>

	<section class="border-b border-border px-5 py-5">
		<button
			type="button"
			onclick={onOpenSf2}
			disabled={!preview?.template || sf2OpenStatus === 'syncing' || !activeClassId}
			class="control-ring inline-flex h-11 w-full items-center justify-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
		>
			{#if sf2OpenStatus === 'syncing'}
				<Spinner />
				Opening...
			{:else}
				<ExternalLink class="size-4" aria-hidden="true" />
				Open SF2 in Excel
			{/if}
		</button>
	</section>

	<section class="border-b border-border px-5 py-5">
		<div class="flex items-start justify-between gap-3">
			<div class="label-mono text-primary">Workbook identity</div>
			<button
				type="button"
				onclick={onEditDetails}
				disabled={!workbookSettings || savingDetails || !activeClassId}
				class="control-ring inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
				title="Edit workbook details"
			>
				<Pencil class="size-3.5" aria-hidden="true" />
				Edit
			</button>
		</div>
		<dl class="mt-4 space-y-3 text-sm">
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
		<!-- Export readiness badge: pulsing skeleton when the grid is re-reading -->
		{#if gridPending}
			<div
				class="mt-4 flex items-center gap-2 rounded-md border border-border bg-background px-3 py-2 text-xs"
			>
				<div
					class="skeleton-pulse size-2 shrink-0 rounded-full bg-muted-foreground/40"
					aria-hidden="true"
				></div>
				<span class="skeleton-pulse text-muted-foreground">Loading preview...</span>
			</div>
		{:else if preview?.canExport !== undefined}
			<div
				class="mt-4 flex items-center gap-2 rounded-md border border-border bg-background px-3 py-2 text-xs"
			>
				<div
					class="size-2 shrink-0 rounded-full {preview.canExport
						? 'bg-emerald-500'
						: 'bg-amber-500'}"
					aria-hidden="true"
				></div>
				<span class="text-muted-foreground">
					{preview.canExport ? 'Ready for export' : 'Needs attention'}
				</span>
			</div>
		{/if}

		<!-- Passive, in place of the old amber recovery panel (spec §12.2). -->
		{#if !gridPending && preview?.template}
			<p class="mt-3 text-xs leading-5 text-muted-foreground">
				{markCounts.text}{#if markCounts.agrees === false}
					<span class="block text-amber-700">
						The workbook and the app disagree. Nothing will be cleared until they agree.
					</span>{/if}
			</p>
		{/if}
	</section>

	<section class="px-5 py-5">
		<div class="flex items-start justify-between gap-3">
			<div>
				<div class="label-mono text-primary">Absent list</div>
				{#if gridPending}
					<h2
						class="skeleton-pulse mt-1 inline-block rounded text-lg font-semibold text-transparent"
					>
						&nbsp;&nbsp;&nbsp;entries
					</h2>
				{:else}
					<h2 class="mt-1 text-lg font-semibold">{preview?.absentList.length ?? 0} entries</h2>
				{/if}
			</div>
			<UserX class="size-5 text-red-700" aria-hidden="true" />
		</div>

		{#if (preview?.absentList.length ?? 0) > 0}
			<div class="mt-4 max-h-80 space-y-2 overflow-auto pr-1">
				{#each preview!.absentList as absence (`${absence.studentId}-${absence.date}`)}
					<div class="rounded-md border border-border bg-background p-3 text-sm">
						<div class="font-medium">{absence.studentName}</div>
						<div class="mt-1 flex items-center justify-between gap-3 text-xs text-muted-foreground">
							<span>{formatDate(absence.date)}</span>
							<span>Row {absence.rowIndex}</span>
						</div>
					</div>
				{/each}
			</div>
		{:else if showNoAbsences}
			<p class="mt-4 text-sm leading-6 text-muted-foreground">
				No absences are currently marked for this report month.
			</p>
		{/if}
	</section>
</aside>

{#snippet metaRow(label: string, value: string)}
	<div class="flex items-center justify-between gap-3">
		<dt class="text-muted-foreground">{label}</dt>
		<dd class="font-medium">{value}</dd>
	</div>
{/snippet}

<style>
	.skeleton-pulse {
		animation: skeleton-pulse 1.5s ease-in-out infinite;
	}

	@keyframes skeleton-pulse {
		0%,
		100% {
			opacity: 0.4;
		}
		50% {
			opacity: 1;
		}
	}
</style>
