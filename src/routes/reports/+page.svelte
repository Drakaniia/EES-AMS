<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import TaskProgress from '$lib/components/ui/TaskProgress.svelte';
	import ReportFilters from './report-filters.svelte';
	import ReportTable from './report-table.svelte';
	import {
		exportSf2Workbook,
		getSf2ExportPreview,
		getSf2WorkbookSettings,
		listClasses,
		openSf2Workbook,
		toggleSf2PreviewAttendance,
		updateSf2WorkbookSettings,
		type Class,
		type Sf2ExportPreview,
		type Sf2PreviewCell,
		type Sf2PreviewStudentRow,
		type Sf2TemplateDraft,
		type Sf2WorkbookSettings
	} from '$lib/db-rust';
	import {
		defaultSf2FirstSchoolDay,
	} from '$lib/features/settings/sf2-workbook';
	import {
		ArrowLeft,
		ExternalLink,
		Maximize2,
		RefreshCw,
		Save,
		Settings2,
		UserX,
	} from 'lucide-svelte';
	import {
		buildMatrixWeekGroups,
		cellKey,
		errorMessage,
		formatImportedAt,
		reportMonthLabel,
		type MatrixStudentRow,
	} from './report-state.svelte';

	let classes = $state<Class[]>([]);
	let selectedClassId = $state('');
	let preview = $state<Sf2ExportPreview | null>(null);
	let workbookSettings = $state<Sf2WorkbookSettings | null>(null);
	let loading = $state(true);
	let loadError = $state<string | null>(null);
	let exporting = $state(false);
	let opening = $state(false);
	let savingDetails = $state(false);
	let correctingCellKey = $state<string | null>(null);
	let exportDialogOpen = $state(false);
	let exportLoadingOpen = $state(false);
	let fullReviewOpen = $state(false);
	let fullReviewHeaderVisible = $state(false);
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	let draftSchoolId = $state('');
	let draftSchoolName = $state('');
	let draftSchoolYear = $state('');
	let draftReportMonth = $state('');
	let draftGradeLevel = $state('');
	let draftSection = $state('');
	let draftAdviserName = $state('');
	let draftSchoolHeadName = $state('');

	onMount(async () => {
		await loadInitial();
	});

	const activeClassId = $derived(
		selectedClassId || preview?.classId || preview?.template?.classId || ''
	);
	const selectedClass = $derived(classes.find((item) => item.id === activeClassId));
	const exportDisabled = $derived(
		!preview?.canExport || exporting || savingDetails || !activeClassId
	);
	const issueCount = $derived(preview?.issues.length ?? 0);
	const warningCount = $derived(preview?.warnings.length ?? 0);
	const activeReportMonth = $derived(draftReportMonth || preview?.template?.reportMonth || '');
	const matrixWeekGroups = $derived(buildMatrixWeekGroups(preview?.dates ?? [], activeReportMonth));
	const matrixStudents = $derived.by((): MatrixStudentRow[] =>
		(preview?.students ?? []).map((row) => ({
			...row,
			cellsByDate: new Map(row.cells.map((cell) => [cell.date, cell]))
		}))
	);

	async function loadInitial() {
		loading = true;
		loadError = null;
		try {
			classes = await listClasses();
			const current = await getSf2ExportPreview();
			preview = current;
			selectedClassId = current.classId ?? classes[0]?.id ?? '';

			if (selectedClassId && selectedClassId !== current.classId) {
				await loadReport(selectedClassId);
			} else {
				await loadWorkbookSettings(current.classId);
			}
		} catch (error) {
			const msg = errorMessage(error, 'Failed to load reports');
			loadError = msg;
			toast(`Reports failed: ${msg}`, false);
		} finally {
			loading = false;
		}
	}

	async function loadReport(classId?: string) {
		const nextPreview = await getSf2ExportPreview(classId);
		preview = nextPreview;
		if (nextPreview.classId) selectedClassId = nextPreview.classId;
		await loadWorkbookSettings(nextPreview.classId ?? classId);
	}

	async function loadWorkbookSettings(classId?: string) {
		if (!classId) {
			workbookSettings = null;
			clearDraft();
			return;
		}

		try {
			const settings = await getSf2WorkbookSettings(classId);
			workbookSettings = settings;
			hydrateDraft(settings);
		} catch {
			workbookSettings = null;
			clearDraft();
		}
	}

	async function onOpenSf2() {
		if (!activeClassId || !preview?.template || opening) return;
		opening = true;
		try {
			const path = await openSf2Workbook(activeClassId);
			toast(`Opened SF2 working copy: ${path}`);
		} catch (error) {
			const msg = errorMessage(error, 'Failed to open SF2');
			toast(`Open SF2 failed: ${msg}`, false);
		} finally {
			opening = false;
		}
	}

	async function requestExport() {
		if (exportDisabled) return;

		const missingFields = blankSf2HeaderFields();
		if (missingFields.length > 0) {
			toast(
				`Fill required SF2 header fields before exporting: ${missingFields.join(', ')}.`,
				false
			);
			return;
		}

		if (hasWorkbookDraftChanges()) {
			const saved = await saveWorkbookDetails(null);
			if (!saved) return;
		}

		exportDialogOpen = true;
	}

	async function confirmExport() {
		if (!activeClassId || !preview?.canExport || exporting) return;
		exportDialogOpen = false;
		exporting = true;
		exportLoadingOpen = true;
		try {
			const result = await exportSf2Workbook(activeClassId);
			toast(`SF2 exported and opened: ${result.outputPath}`);
			await loadReport(activeClassId);
		} catch (error) {
			const msg = errorMessage(error, 'SF2 export failed');
			toast(`SF2 export failed: ${msg}`, false);
		} finally {
			exporting = false;
			exportLoadingOpen = false;
		}
	}

	async function saveWorkbookDetails(successMessage: string | null = 'SF2 workbook details saved') {
		const draft = workbookDraftPayload();
		if (!draft || savingDetails) return false;

		savingDetails = true;
		try {
			await updateSf2WorkbookSettings(draft);
			if (successMessage) toast(successMessage);
			await loadReport(draft.classId);
			return true;
		} catch (error) {
			const msg = errorMessage(error, 'SF2 workbook update failed');
			toast(`SF2 workbook update failed: ${msg}`, false);
			return false;
		} finally {
			savingDetails = false;
		}
	}

	async function onReportMonthChange() {
		const previousReportMonth =
			workbookSettings?.reportMonth || preview?.template?.reportMonth || '';
		const saved = await saveWorkbookDetails('SF2 report month updated');
		if (!saved) draftReportMonth = previousReportMonth;
	}

	function onWindowKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && fullReviewOpen) {
			fullReviewOpen = false;
		}
	}

	async function toggleAttendance(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
		if (!preview?.classId || !row.mapped || !cell.editable || correctingCellKey) return;
		const key = cellKey(row.studentId, cell.date);
		const markPresent = cell.status !== 'present';
		correctingCellKey = key;
		try {
			// Lightweight DB-only toggle — no Excel I/O, no preview rebuild.
			// Click the Refresh button above to reload the full preview.
			await toggleSf2PreviewAttendance(
				preview.classId,
				row.studentId,
				cell.date,
				markPresent
			);
			toast(
				`${row.studentName} marked ${markPresent ? 'present' : 'absent'} for ${formatDate(cell.date)}`
			);
		} catch (error) {
			const msg = errorMessage(error, 'Attendance correction failed');
			toast(`Attendance correction failed: ${msg}`, false);
		} finally {
			correctingCellKey = null;
		}
	}

	function blankSf2HeaderFields() {
		return sf2HeaderFields()
			.filter((field) => field.value.trim() === '')
			.map((field) => field.label);
	}

	function sf2HeaderFields() {
		return [
			{ label: 'School ID', value: draftSchoolId },
			{ label: 'Name of School', value: draftSchoolName },
			{ label: 'School Year', value: draftSchoolYear },
			{ label: 'Report Month', value: draftReportMonth },
			{ label: 'Grade Level', value: draftGradeLevel },
			{ label: 'Section', value: draftSection },
			{ label: 'Adviser / LIS Name', value: draftAdviserName },
			{ label: 'School Head Name', value: draftSchoolHeadName }
		];
	}

	function hasWorkbookDraftChanges() {
		if (!workbookSettings) return false;
		return (
			draftSchoolId !== workbookSettings.schoolId ||
			draftSchoolName !== workbookSettings.schoolName ||
			draftSchoolYear !== workbookSettings.schoolYear ||
			draftReportMonth !== workbookSettings.reportMonth ||
			draftGradeLevel !== workbookSettings.gradeLevel ||
			draftSection !== workbookSettings.section ||
			draftAdviserName !== workbookSettings.adviserName ||
			draftSchoolHeadName !== workbookSettings.schoolHeadName
		);
	}

	function workbookDraftPayload(): Sf2TemplateDraft | null {
		if (!workbookSettings || !activeClassId) return null;
		return {
			classId: activeClassId,
			schoolId: draftSchoolId,
			schoolName: draftSchoolName,
			schoolYear: draftSchoolYear,
			reportMonth: draftReportMonth,
			gradeLevel: draftGradeLevel,
			section: draftSection,
			adviserName: draftAdviserName,
			schoolHeadName: draftSchoolHeadName,
			firstSchoolDay: defaultSf2FirstSchoolDay(draftReportMonth, draftSchoolYear),
			learnerNames: workbookSettings.learnerNames
		};
	}

	function hydrateDraft(settings: Sf2WorkbookSettings) {
		draftSchoolId = settings.schoolId;
		draftSchoolName = settings.schoolName;
		draftSchoolYear = settings.schoolYear;
		draftReportMonth = settings.reportMonth;
		draftGradeLevel = settings.gradeLevel;
		draftSection = settings.section;
		draftAdviserName = settings.adviserName;
		draftSchoolHeadName = settings.schoolHeadName;
	}

	function clearDraft() {
		draftSchoolId = '';
		draftSchoolName = '';
		draftSchoolYear = '';
		draftReportMonth = '';
		draftGradeLevel = '';
		draftSection = '';
		draftAdviserName = '';
		draftSchoolHeadName = '';
	}

	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 4000);
	}

	function formatDate(date: string) {
		const value = new Date(`${date}T00:00:00`);
		return value.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
	}

	function onDraftChange(field: string, value: string) {
		if (field === 'draftSchoolId') draftSchoolId = value;
		else if (field === 'draftSchoolName') draftSchoolName = value;
		else if (field === 'draftSchoolYear') draftSchoolYear = value;
		else if (field === 'draftReportMonth') draftReportMonth = value;
		else if (field === 'draftGradeLevel') draftGradeLevel = value;
		else if (field === 'draftSection') draftSection = value;
		else if (field === 'draftAdviserName') draftAdviserName = value;
		else if (field === 'draftSchoolHeadName') draftSchoolHeadName = value;
	}
</script>

<svelte:head>
	<title>Reports - Attendance System</title>
	<meta name="description" content="Review and export DepEd SF2 Excel reports." />
</svelte:head>

<svelte:window onkeydown={onWindowKeydown} />

<div class="flex h-full flex-col overflow-hidden">
	<PageHeader
		category="Reports"
		title="SF2 Workbook Pre-export Review"
		description="Review workbook details, mapped dates, absences, and learner row mappings before exporting the official SF2 copy."
	>
		{#snippet actions()}
			<div class="flex flex-wrap items-center gap-2">
				<button
					type="button"
					onclick={loadInitial}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface"
				>
					<RefreshCw class="size-4" aria-hidden="true" />
					Refresh
				</button>
				<button
					type="button"
					onclick={onOpenSf2}
					disabled={!preview?.template || opening || !activeClassId}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
				>
					<ExternalLink class="size-4" aria-hidden="true" />
					{opening ? 'Opening...' : 'Open SF2'}
				</button>
				<button
					type="button"
					onclick={() => (fullReviewOpen = true)}
					disabled={!preview?.template}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
				>
					<Maximize2 class="size-4" aria-hidden="true" />
					View Full Review
				</button>
				<button
					type="button"
					onclick={requestExport}
					disabled={exportDisabled}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
				>
					<Save class="size-4" aria-hidden="true" />
					{exporting ? 'Exporting...' : 'Review Export'}
				</button>
			</div>
		{/snippet}
	</PageHeader>

	{#if loading}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<LoadingBlock rows={4} label="Loading SF2 workbook preview" />
		</div>
	{:else if loadError}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<EmptyState tone="warning" title="SF2 reports are unavailable" description={loadError}>
				{#snippet actions()}
					<button
						type="button"
						onclick={loadInitial}
						class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
					>
						Retry
					</button>
				{/snippet}
			</EmptyState>
		</div>
	{:else if !preview?.template}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<EmptyState
				tone="warning"
				title="No SF2 workbook is ready for review"
				description={preview?.issues[0] ??
					'Import an SF2 workbook or create one from the bundled template first.'}
			>
				{#snippet actions()}
					<a
						href={resolve('/settings')}
						class="control-ring inline-flex rounded-pill bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground hover:bg-accent"
					>
						Open SF2 Settings
					</a>
				{/snippet}
			</EmptyState>
		</div>
	{:else}
		<section
			class="grid min-h-0 flex-1 gap-5 overflow-hidden px-4 py-5 md:px-8 lg:px-10 xl:grid-cols-[minmax(0,1fr)_360px]"
		>
			<div class="min-h-0 space-y-5 overflow-auto pr-0 xl:pr-1">
				<ReportFilters
					{preview}
					{selectedClass}
					{issueCount}
					{warningCount}
					fullReview={false}
					{fullReviewHeaderVisible}
					{workbookSettings}
					{savingDetails}
					{draftSchoolId}
					{draftSchoolName}
					{draftSchoolYear}
					{draftReportMonth}
					{draftGradeLevel}
					{draftSection}
					{draftAdviserName}
					{draftSchoolHeadName}
					onSaveWorkbookDetails={saveWorkbookDetails}
					{onReportMonthChange}
					{onDraftChange}
				/>

				<ReportTable
					previewTemplateGradeLevel={preview.template.gradeLevel}
					previewTemplateSection={preview.template.section}
					{matrixWeekGroups}
					{matrixStudents}
					{correctingCellKey}
					fullReview={false}
					onToggleAttendance={toggleAttendance}
				/>
			</div>

			<aside class="min-h-0 space-y-5 overflow-auto">
				<div class="rounded-2xl border border-border bg-surface p-5">
					<div class="label-mono text-primary">Workbook identity</div>
					<dl class="mt-4 space-y-3 text-sm">
						{@render metaRow('Class', preview.className || selectedClass?.name || 'Unlinked')}
						{@render metaRow('Report Month', reportMonthLabel(preview.template.reportMonth))}
						{@render metaRow('School ID', preview.template.schoolId || 'Blank')}
						{@render metaRow('School Year', preview.template.schoolYear || 'Blank')}
						{@render metaRow('Adviser', preview.template.adviserName || 'Blank')}
						{@render metaRow('School Head', preview.template.schoolHeadName || 'Blank')}
						{@render metaRow('Imported', formatImportedAt(preview.template.importedAt))}
					</dl>
				</div>

				<div class="rounded-2xl border border-border bg-card p-5">
					<div class="flex items-start justify-between gap-3">
						<div>
							<div class="label-mono text-primary">Absent list</div>
							<h2 class="mt-1 text-lg font-semibold">{preview.absentList.length} entries</h2>
						</div>
						<UserX class="size-5 text-red-700" aria-hidden="true" />
					</div>

					{#if preview.absentList.length > 0}
						<div class="mt-4 max-h-80 space-y-2 overflow-auto pr-1">
							{#each preview.absentList as absence (`${absence.studentId}-${absence.date}`)}
								<div class="rounded-md border border-border bg-background p-3 text-sm">
									<div class="font-medium">{absence.studentName}</div>
									<div
										class="mt-1 flex items-center justify-between gap-3 text-xs text-muted-foreground"
									>
										<span>{formatDate(absence.date)}</span>
										<span>Row {absence.rowIndex}</span>
									</div>
								</div>
							{/each}
						</div>
					{:else}
						<p class="mt-4 text-sm leading-6 text-muted-foreground">
							No absences are currently marked for this report month.
						</p>
					{/if}
				</div>
			</aside>
		</section>
	{/if}
</div>

{#if fullReviewOpen && preview?.template}
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Full SF2 review"
		class="fixed inset-x-0 top-8 bottom-0 z-[65] bg-background text-foreground"
	>
		<div class="flex h-full min-h-0 flex-col overflow-hidden">
			<div
				class="flex shrink-0 items-center justify-between gap-3 border-b border-border bg-background px-4 py-3 md:px-6"
			>
				<button
					type="button"
					onclick={() => (fullReviewOpen = false)}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface"
				>
					<ArrowLeft class="size-4" aria-hidden="true" />
					Back
				</button>
				<div class="min-w-0 text-right">
					<div class="label-mono text-primary">Full review</div>
					<div class="truncate text-sm font-semibold">
						{draftSchoolName || preview.template.schoolName || 'SF2 Workbook'}
					</div>
				</div>
				<button
					type="button"
					onclick={() => (fullReviewHeaderVisible = !fullReviewHeaderVisible)}
					class="control-ring inline-flex h-10 items-center gap-2 rounded-md border border-border bg-background px-3.5 text-sm font-medium transition-colors hover:bg-surface"
				>
					<Settings2 class="size-4" aria-hidden="true" />
					{fullReviewHeaderVisible ? 'Hide Details' : 'Show Details'}
				</button>
			</div>

			<div class="min-h-0 flex-1 overflow-hidden px-4 py-4 md:px-6">
				<div class="flex h-full min-h-0 flex-col gap-4">
					{#if fullReviewHeaderVisible}
						<ReportFilters
							{preview}
							{selectedClass}
							{issueCount}
							{warningCount}
							fullReview={true}
							{fullReviewHeaderVisible}
							{workbookSettings}
							{savingDetails}
							{draftSchoolId}
							{draftSchoolName}
							{draftSchoolYear}
							{draftReportMonth}
							{draftGradeLevel}
							{draftSection}
							{draftAdviserName}
							{draftSchoolHeadName}
							onSaveWorkbookDetails={saveWorkbookDetails}
							{onReportMonthChange}
							{onDraftChange}
						/>
					{/if}
					<ReportTable
						previewTemplateGradeLevel={preview.template.gradeLevel}
						previewTemplateSection={preview.template.section}
						{matrixWeekGroups}
						{matrixStudents}
						{correctingCellKey}
						fullReview={true}
						onToggleAttendance={toggleAttendance}
					/>
				</div>
			</div>
		</div>
	</div>
{/if}

<Dialog
	open={exportDialogOpen}
	title="Confirm SF2 Export"
	description="Export copies the reviewed SF2 working workbook to your chosen file path."
	maxWidth="2xl"
	onClose={() => (exportDialogOpen = false)}
>
	<div class="grid gap-3 sm:grid-cols-2">
		{@render confirmStat('Mapped learners', preview?.mappedStudents ?? 0)}
		{@render confirmStat('Absences', preview?.absenceCount ?? 0)}
	</div>

	{#if preview && preview.warnings.length > 0}
		<div class="rounded-md border border-amber-500/30 bg-amber-50 p-4 text-sm text-amber-900">
			<div class="font-semibold">Review these warnings before exporting.</div>
			<ul class="mt-3 max-h-48 space-y-2 overflow-auto">
				{#each preview.warnings as warning, index (`confirm-warning-${index}-${warning}`)}
					<li>{warning}</li>
				{/each}
			</ul>
		</div>
	{:else}
		<div class="rounded-md border border-emerald-500/30 bg-emerald-50 p-4 text-sm text-emerald-800">
			The workbook details, date mappings, and learner mappings have no detected warnings.
		</div>
	{/if}

	<div class="flex flex-wrap justify-end gap-2">
		<button
			type="button"
			onclick={() => (exportDialogOpen = false)}
			class="control-ring h-10 rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-surface"
		>
			Cancel
		</button>
		<button
			type="button"
			onclick={confirmExport}
			disabled={exporting || !preview?.canExport}
			class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
		>
			<Save class="size-4" aria-hidden="true" />
			{exporting ? 'Exporting...' : 'Export Workbook'}
		</button>
	</div>
</Dialog>

<Dialog
	open={exportLoadingOpen}
	title="Exporting SF2 Workbook"
	description="Saving the reviewed workbook and opening the exported file."
	maxWidth="lg"
	showCloseButton={false}
>
	<TaskProgress
		active={exportLoadingOpen}
		title="Exporting SF2 workbook"
		description="Writing attendance marks, copying the workbook, and opening the generated file."
		simple
	/>
</Dialog>

<FeedbackToast message={toastMessage} ok={toastOk} onClose={() => (toastMessage = null)} />

{#snippet confirmStat(label: string, value: number)}
	<div class="rounded-md border border-border bg-surface p-3">
		<div class="label-mono">{label}</div>
		<div class="mt-2 text-2xl font-semibold">{value}</div>
	</div>
{/snippet}

{#snippet metaRow(label: string, value: string)}
	<div class="flex items-center justify-between gap-3">
		<dt class="text-muted-foreground">{label}</dt>
		<dd class="font-medium">{value}</dd>
	</div>
{/snippet}
