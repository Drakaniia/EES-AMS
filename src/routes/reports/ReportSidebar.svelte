<script lang="ts">
	import { Calendar, ExternalLink, Pencil, RefreshCw, Save, UserX } from 'lucide-svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { reportMonthLabel, formatImportedAt, formatDate } from './report-state.svelte';
	import type { Sf2ExportPreview, Sf2WorkbookSettings } from '$lib/db-rust';
	import type { Class } from '$lib/db-rust';

	type Props = {
		preview: Sf2ExportPreview | null;
		previewRefreshing: boolean;
		selectedClass: Class | null | undefined;
		draftSchoolId: string;
		draftSchoolYear: string;
		draftReportMonth: string;
		draftGradeLevel: string;
		draftSection: string;
		draftAdviserName: string;
		draftSchoolHeadName: string;
		exportDisabled: boolean;
		exporting: boolean;
		syncingRoster: boolean;
		sf2OpenStatus: string;
		workbookSettings: Sf2WorkbookSettings | null;
		savingDetails: boolean;
		activeClassId: string;
		onOpenSf2?: () => void;
		onSyncRoster?: () => void;
		onRequestExport?: () => void;
		onEditDetails?: () => void;
		onSwitchMonth?: () => void;
	};

	let {
		preview,
		previewRefreshing = false,
		selectedClass,
		draftSchoolId,
		draftSchoolYear,
		draftReportMonth,
		draftGradeLevel,
		draftSection,
		draftAdviserName,
		draftSchoolHeadName,
		exportDisabled,
		exporting,
		syncingRoster,
		sf2OpenStatus,
		workbookSettings,
		savingDetails,
		activeClassId,
		onOpenSf2,
		onSyncRoster,
		onRequestExport,
		onEditDetails,
		onSwitchMonth
	}: Props = $props();
</script>

<aside class="min-h-0 space-y-5 overflow-auto">
	<div class="rounded-2xl border border-border bg-surface p-5">
		<div class="label-mono mb-4 text-primary">Actions</div>
		<div class="flex flex-col gap-2">
			<button
				type="button"
				onclick={onOpenSf2}
				disabled={!preview?.template || sf2OpenStatus === 'syncing' || !activeClassId}
				class="control-ring inline-flex h-10 w-full items-center justify-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
			>
				<ExternalLink class="size-4" aria-hidden="true" />
				{sf2OpenStatus === 'syncing' ? 'Opening...' : 'Open SF2'}
			</button>
			<button
				type="button"
				onclick={onSyncRoster}
				disabled={!preview?.template || syncingRoster || !activeClassId}
				class="control-ring inline-flex h-10 w-full items-center justify-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
				aria-label="Sync class roster to SF2 workbook"
			>
				{#if syncingRoster}
					<Spinner />
				{:else}
					<RefreshCw class="size-4" aria-hidden="true" />
				{/if}
				{syncingRoster ? 'Syncing...' : 'Sync Roster'}
			</button>
			<!-- Export button: show skeleton pulsing when preview is refreshing -->
			{#if previewRefreshing}
				<button
					type="button"
					disabled
					class="control-ring inline-flex h-10 w-full cursor-not-allowed items-center justify-center gap-2 rounded-pill bg-primary/60 px-4 text-sm font-semibold text-primary-foreground/70"
				>
					<Spinner />
					Loading preview...
				</button>
			{:else}
				<button
					type="button"
					onclick={onRequestExport}
					disabled={exportDisabled}
					class="control-ring inline-flex h-10 w-full items-center justify-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
				>
					<Save class="size-4" aria-hidden="true" />
					{exporting ? 'Exporting...' : 'Review Export'}
				</button>
			{/if}
		</div>
	</div>

	<div class="rounded-2xl border border-border bg-surface p-5">
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
			<button
				type="button"
				onclick={onSwitchMonth}
				disabled={!workbookSettings || !activeClassId}
				class="control-ring inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
				title="Switch SF2 report month"
			>
				<Calendar class="size-3.5" aria-hidden="true" />
				Switch month
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
		<!-- Export readiness badge: pulsing skeleton when refreshing, live indicator otherwise -->
		{#if previewRefreshing}
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
	</div>

	<div class="rounded-2xl border border-border bg-card p-5">
		<div class="flex items-start justify-between gap-3">
			<div>
				<div class="label-mono text-primary">Absent list</div>
				{#if previewRefreshing}
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
		{:else if !previewRefreshing}
			<p class="mt-4 text-sm leading-6 text-muted-foreground">
				No absences are currently marked for this report month.
			</p>
		{/if}
	</div>
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
