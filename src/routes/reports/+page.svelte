<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount, onDestroy } from 'svelte';
	import { fade } from 'svelte/transition';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import ReportFilters from './report-filters.svelte';
	import ReportTable from './report-table.svelte';
	import ReportExportDialogs from './report-export-dialogs.svelte';
	import {
		exportSf2Workbook,
		getSf2ExportPreview,
		getSf2WorkbookSettings,
		listClasses,
		presentAllSf2PreviewAttendance,
		setSf2ReportMonth,
		syncAndOpenSf2Workbook,
		syncSf2Roster,
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
		normalizedSf2FirstSchoolDay,
		SF2_SCHOOL_MONTHS
	} from '$lib/features/settings/sf2-workbook';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import {
		ArrowLeft,
		Calendar,
		Check,
		CheckCircle2,
		CircleX,
		ExternalLink,
		Pencil,
		RefreshCw,
		Save,
		Settings2,
		UserX
	} from 'lucide-svelte';
	import {
		buildMatrixWeekGroups,
		cellKey,
		errorMessage,
		formatImportedAt,
		reportMonthLabel,
		type MatrixStudentRow
	} from './report-state.svelte';

	// ── Friendly loading messages that cycle during SF2 open ────────────────
	const SF2_OPEN_MESSAGES = [
		'Warming up the workbook…',
		'Reading attendance records…',
		'Writing marks to the workbook…',
		'Almost there, wrapping things up…',
		'Opening in Excel…',
		'Double-checking everything is in order…',
		'Just a moment longer!'
	] as const;

	// ── Preview cache: eliminates redundant backend calls on month switch ───
	// When switching to a month that's already been loaded, returns instantly.
	// Key format: `${classId}:${reportMonth}`
	const previewCache = new Map<string, Sf2ExportPreview>();

	function cacheKey(classId: string, reportMonth: string): string {
		return `${classId}:${reportMonth}`;
	}

	function invalidateCacheForMonth(classId: string, reportMonth: string) {
		previewCache.delete(cacheKey(classId, reportMonth));
	}

	function invalidateAllCache() {
		previewCache.clear();
	}

	// ── Progress state for the SF2 open dialog ──────────────────────────────
	type Sf2OpenStatus = 'idle' | 'syncing' | 'success' | 'error';
	let sf2OpenStatus = $state<Sf2OpenStatus>('idle');
	let sf2OpenProgressCurrent = $state(0);
	let sf2OpenProgressTotal = $state(10);
	let sf2OpenMessage = $state('');
	let sf2OpenError = $state<string | null>(null);
	let sf2OpenResultPath = $state<string | null>(null);

	// Cycling messages state
	let sf2OpenCycleIndex = $state(0);
	let sf2OpenLastBackendMsg = $state('');
	let sf2OpenLastBackendTime = $state(0);
	let sf2OpenDisplayMessage = $state('');
	let sf2OpenCycleTimer: ReturnType<typeof setInterval> | null = null;
	let sf2OpenSuccessTimer: ReturnType<typeof setTimeout> | null = null;
	let sf2OpenUnlisten: UnlistenFn | null = null;

	let classes = $state<Class[]>([]);
	let selectedClassId = $state('');
	let preview = $state<Sf2ExportPreview | null>(null);
	let workbookSettings = $state<Sf2WorkbookSettings | null>(null);
	let loading = $state(true);
	let loadError = $state<string | null>(null);
	let genderFilter = $state<'all' | 'male' | 'female'>('all');
	let exporting = $state(false);
	let syncingRoster = $state(false);
	let presentingAll = $state(false);
	let savingDetails = $state(false);
	let correctingCellKey = $state<string | null>(null);
	let exportDialogOpen = $state(false);
	let exportLoadingOpen = $state(false);
	let fullReviewOpen = $state(false);
	let fullReviewHeaderVisible = $state(false);
	let workbookDetailsOpen = $state(false);
	let monthPickerOpen = $state(false);
	let monthSwitchLoading = $state(false);
	let monthSwitchError = $state<string | null>(null);
	let modalSaving = $state(false);
	let reportDialogs: ReportExportDialogs;

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

	onDestroy(() => {
		cleanupSf2Open();
	});

	// ── SF2 Open progress helpers ───────────────────────────────────────────

	function startSf2MessageCycle() {
		stopSf2MessageCycle();
		updateSf2DisplayMessage();
		sf2OpenCycleTimer = setInterval(() => {
			const now = Date.now();
			// Only cycle if no backend message arrived in the last 3 seconds
			if (now - sf2OpenLastBackendTime > 3000) {
				sf2OpenCycleIndex = (sf2OpenCycleIndex + 1) % SF2_OPEN_MESSAGES.length;
			}
			updateSf2DisplayMessage();
		}, 2500);
	}

	function stopSf2MessageCycle() {
		if (sf2OpenCycleTimer !== null) {
			clearInterval(sf2OpenCycleTimer);
			sf2OpenCycleTimer = null;
		}
	}

	/// Compute the current display message using $state so it's always reactive.
	/// Called from the cycle timer, the progress event listener, and on reset.
	/// Previously used $derived + Date.now() which was NOT reactive.
	function updateSf2DisplayMessage() {
		// Backend message has priority for 4 seconds
		if (sf2OpenLastBackendMsg && Date.now() - sf2OpenLastBackendTime < 4000) {
			sf2OpenDisplayMessage = sf2OpenLastBackendMsg;
			return;
		}
		// Map progress steps to messages when no backend message
		if (sf2OpenProgressCurrent > 0 && sf2OpenProgressTotal > 0) {
			const progressMessages: Record<number, string> = {
				1: 'Warming up the workbook…',
				2: 'Reading attendance records…',
				3: 'Checking date mappings…',
				4: 'Clearing previous marks…',
				5: 'Computing attendance marks…',
				6: 'Writing marks to the workbook…',
				7: 'Saving workbook changes…',
				8: 'Preparing to open…',
				9: 'Opening in Excel…',
				10: 'Done!'
			};
			sf2OpenDisplayMessage =
				progressMessages[sf2OpenProgressCurrent] || SF2_OPEN_MESSAGES[sf2OpenCycleIndex];
			return;
		}
		sf2OpenDisplayMessage = SF2_OPEN_MESSAGES[sf2OpenCycleIndex];
	}

	const sf2OpenProgressPercent = $derived.by(() => {
		if (sf2OpenProgressTotal <= 0) return 0;
		return Math.round((sf2OpenProgressCurrent / sf2OpenProgressTotal) * 100);
	});

	async function setupSf2ProgressListener() {
		cleanupSf2Open();
		try {
			sf2OpenUnlisten = await listen<{
				task: string;
				current: number;
				total: number;
				message: string;
			}>('sf2-progress', (event) => {
				if (event.payload.task === 'open') {
					sf2OpenProgressCurrent = event.payload.current;
					sf2OpenProgressTotal = event.payload.total;
					if (event.payload.message) {
						sf2OpenLastBackendMsg = event.payload.message;
						sf2OpenLastBackendTime = Date.now();
					}
					updateSf2DisplayMessage();
				}
			});
		} catch {
			// Listener setup failed; continue without it (indeterminate fallback)
		}
	}

	function cleanupSf2Open() {
		stopSf2MessageCycle();
		if (sf2OpenSuccessTimer !== null) {
			clearTimeout(sf2OpenSuccessTimer);
			sf2OpenSuccessTimer = null;
		}
		if (sf2OpenUnlisten) {
			sf2OpenUnlisten();
			sf2OpenUnlisten = null;
		}
	}

	async function onOpenSf2() {
		if (!activeClassId || !preview?.template || sf2OpenStatus === 'syncing') return;

		// Reset progress state
		sf2OpenStatus = 'syncing';
		sf2OpenProgressCurrent = 0;
		sf2OpenProgressTotal = 10;
		sf2OpenMessage = '';
		sf2OpenError = null;
		sf2OpenResultPath = null;
		sf2OpenCycleIndex = 0;
		sf2OpenLastBackendMsg = '';
		sf2OpenLastBackendTime = 0;

		// Show first step immediately for responsiveness
		sf2OpenProgressCurrent = 1;
		sf2OpenCycleIndex = 0;

		// Set up progress event listener
		await setupSf2ProgressListener();
		startSf2MessageCycle();

		try {
			const path = await syncAndOpenSf2Workbook(activeClassId);
			sf2OpenResultPath = path;
			sf2OpenStatus = 'success';
			stopSf2MessageCycle();
			reportDialogs?.showToast(`Opened SF2 working copy: ${path}`);

			// Auto-close after 1.5 seconds
			sf2OpenSuccessTimer = setTimeout(() => {
				sf2OpenStatus = 'idle';
			}, 1500);
		} catch (error) {
			stopSf2MessageCycle();
			const msg = errorMessage(error, 'Failed to update SF2 workbook');
			if (msg.toLowerCase().includes('excel')) {
				sf2OpenError =
					'The SF2 working copy is currently open in Microsoft Excel. ' +
					'Close the workbook in Excel first, then click Open SF2 again.';
			} else {
				sf2OpenError = `Could not sync attendance to the SF2 workbook: ${msg}`;
			}
			sf2OpenStatus = 'error';
		}
	}

	async function retrySf2Open() {
		sf2OpenStatus = 'idle';
		sf2OpenError = null;
		// Small delay so the UI resets cleanly before re-triggering
		await new Promise((resolve) => setTimeout(resolve, 50));
		await onOpenSf2();
	}

	const activeClassId = $derived(
		selectedClassId || preview?.classId || preview?.template?.classId || ''
	);
	const selectedClass = $derived(classes.find((item) => item.id === activeClassId));
	const exportDisabled = $derived(
		!preview?.canExport || exporting || savingDetails || !activeClassId
	);
	const activeReportMonth = $derived(draftReportMonth || preview?.template?.reportMonth || '');
	const matrixWeekGroups = $derived(buildMatrixWeekGroups(preview?.dates ?? [], activeReportMonth));
	const matrixStudents = $derived.by((): MatrixStudentRow[] =>
		(preview?.students ?? [])
			.filter((row) => genderFilter === 'all' || row.gender?.toLowerCase() === genderFilter)
			.map((row) => ({
				...row,
				cellsByDate: new Map(row.cells.map((cell) => [cell.date, cell]))
			}))
	);

	async function loadInitial() {
		loading = true;
		loadError = null;
		try {
			classes = await listClasses();
			// Fetch initial preview and cache it
			const current = await getSf2ExportPreview();
			preview = current;
			selectedClassId = current.classId ?? classes[0]?.id ?? '';

			// Cache the initial preview if we have a class + report month
			if (current.classId && current.template?.reportMonth) {
				previewCache.set(cacheKey(current.classId, current.template.reportMonth), current);
			}

			if (selectedClassId && selectedClassId !== current.classId) {
				await loadReport(selectedClassId);
			} else {
				await loadWorkbookSettings(current.classId);
			}
		} catch (error) {
			const msg = errorMessage(error, 'Failed to load reports');
			loadError = msg;
			reportDialogs?.showToast(`Reports failed: ${msg}`, false);
		} finally {
			loading = false;
		}
	}

	/// Load preview + workbook settings, checking cache first and parallelizing IPC.
	/// - Checks client-side cache keyed by (classId, reportMonth) to skip backend calls
	/// - Fetches preview and workbook settings in parallel via Promise.all
	/// - Caches the preview result for instant re-visits to the same month
	async function loadReport(classId?: string) {
		const cid = classId || selectedClassId || preview?.classId || preview?.template?.classId || '';
		if (!cid) return;

		const reportMonth = activeReportMonth;
		const key = cacheKey(cid, reportMonth);

		// Check cache first — instant return on repeat month switches
		const cached = previewCache.get(key);
		if (cached) {
			preview = cached;
			if (cached.classId) selectedClassId = cached.classId;
			await loadWorkbookSettings(cached.classId ?? cid);
			return;
		}

		// Parallel fetch: both preview and settings are independent
		const [nextPreview, settings] = await Promise.all([
			getSf2ExportPreview(classId),
			cid ? getSf2WorkbookSettings(cid).catch(() => null) : Promise.resolve(null)
		]);

		preview = nextPreview;
		if (nextPreview.classId) selectedClassId = nextPreview.classId;

		// Cache the preview keyed by class + report month
		const cacheMonth = nextPreview.template?.reportMonth || reportMonth;
		if (cacheMonth) {
			previewCache.set(cacheKey(cid, cacheMonth), nextPreview);
		}

		if (settings) {
			workbookSettings = settings;
			hydrateDraft(settings);
		} else {
			workbookSettings = null;
			clearDraft();
		}
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

	const hasAbsentCells = $derived((preview?.absentList.length ?? 0) > 0);

	async function onPresentAll() {
		if (!activeClassId || !preview?.template || presentingAll) return;
		presentingAll = true;
		try {
			const count = await presentAllSf2PreviewAttendance(activeClassId);
			// Invalidate cache since attendance data changed
			invalidateCacheForMonth(activeClassId, activeReportMonth);
			reportDialogs?.showToast(`All students cleared to Present (${count} marks cleared)`);
			await loadReport(activeClassId);
		} catch (error) {
			const msg = errorMessage(error, 'Present All failed');
			reportDialogs?.showToast(`Could not mark all present: ${msg}`, false);
		} finally {
			presentingAll = false;
		}
	}

	async function onSyncRoster() {
		if (!activeClassId || !preview?.template || syncingRoster) return;
		syncingRoster = true;
		try {
			await syncSf2Roster(activeClassId);
			// Roster changed — invalidate ALL cached months
			invalidateAllCache();
			reportDialogs?.showToast('Roster synced! All students mapped to SF2 workbook.');
			await loadReport(activeClassId);
		} catch (error) {
			const msg = errorMessage(error, 'Roster sync failed');
			reportDialogs?.showToast(`Could not sync roster: ${msg}`, false);
		} finally {
			syncingRoster = false;
		}
	}

	async function requestExport() {
		if (exportDisabled) return;

		const missingFields = blankSf2HeaderFields();
		if (missingFields.length > 0) {
			reportDialogs?.showToast(
				`Fill required SF2 header fields before exporting: ${missingFields.join(', ')}.`,
				false
			);
			return;
		}

		if (hasModalDraftChanges) {
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
			// Export may have written marks — invalidate cache for current month
			invalidateCacheForMonth(activeClassId, activeReportMonth);
			reportDialogs?.showToast(`SF2 exported and opened: ${result.outputPath}`);
			await loadReport(activeClassId);
		} catch (error) {
			const msg = errorMessage(error, 'SF2 export failed');
			reportDialogs?.showToast(`SF2 export failed: ${msg}`, false);
		} finally {
			exporting = false;
			exportLoadingOpen = false;
		}
	}

	async function saveWorkbookDetails(successMessage: string | null = 'SF2 workbook details saved') {
		if (!activeClassId || savingDetails || modalSaving) return false;
		const draft = workbookDraftPayload();
		if (!draft || savingDetails) return false;
		savingDetails = true;
		modalSaving = true;
		try {
			await updateSf2WorkbookSettings(draft);
			// Workbook settings affect all months — invalidate entire cache
			invalidateAllCache();
			if (successMessage) reportDialogs?.showToast(successMessage);
			await loadReport(draft.classId);
			return true;
		} catch (error) {
			const msg = errorMessage(error, 'SF2 workbook update failed');
			reportDialogs?.showToast(`SF2 workbook update failed: ${msg}`, false);
			return false;
		} finally {
			savingDetails = false;
			modalSaving = false;
		}
	}

	async function onReportMonthChange() {
		const previousReportMonth =
			workbookSettings?.reportMonth || preview?.template?.reportMonth || '';
		const nextMonth = draftReportMonth;
		if (!nextMonth || nextMonth === previousReportMonth) return;

		// Lightweight switch: update only the active report month in the DB.
		// This deliberately avoids the heavy Excel automation that a full
		// settings save would trigger, so switching months is instant and
		// cannot fail with "Excel automation failed" when Excel is locked.
		if (!activeClassId) {
			draftReportMonth = previousReportMonth;
			return;
		}

		// Invalidate the OLD month's cache before switching so we don't serve
		// stale preview data when switching back to it.
		invalidateCacheForMonth(activeClassId, previousReportMonth);

		monthSwitchLoading = true;
		monthSwitchError = null;
		const switchStartTime = Date.now();
		try {
			await setSf2ReportMonth(activeClassId, nextMonth);
			await loadReport(activeClassId);
			reportDialogs?.showToast(`Switched to ${reportMonthLabel(nextMonth)}`);
		} catch (error) {
			const msg = errorMessage(error, 'Failed to switch report month');
			monthSwitchError = msg;
			reportDialogs?.showToast(`Could not switch month: ${msg}`, false);
			draftReportMonth = previousReportMonth;
		} finally {
			// Ensure the progress dialog is visible for at least 500ms to
			// prevent a jarring flash for fast switches.
			const elapsed = Date.now() - switchStartTime;
			if (elapsed < 500) {
				await new Promise((resolve) => setTimeout(resolve, 500 - elapsed));
			}
			monthSwitchLoading = false;
		}
	}

	function onWindowKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && fullReviewOpen) {
			fullReviewOpen = false;
		}
	}

	async function toggleAttendance(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
		if (!preview?.classId || !row.mapped || !cell.editable || correctingCellKey) return;
		const key = cellKey(row.studentId, cell.date);
		// Clicking Present (empty) or Open (-) should mark as Absent (X).
		// Clicking Absent (X) should mark as Present (empty).
		const markPresent = cell.status === 'absent';
		correctingCellKey = key;
		try {
			// Lightweight DB-only toggle — no Excel I/O.
			// Open SF2 will sync attendance before opening the workbook.
			await toggleSf2PreviewAttendance(preview.classId, row.studentId, cell.date, markPresent);
			// Invalidate cache for current month since attendance changed
			invalidateCacheForMonth(preview.classId, activeReportMonth);
			// Reload preview from DB so the grid reflects the change instantly
			await loadReport(activeClassId);
			reportDialogs?.showToast(
				`${row.studentName} marked ${markPresent ? 'present' : 'absent'} for ${formatDate(cell.date)}`
			);
		} catch (error) {
			const msg = errorMessage(error, 'Attendance correction failed');
			reportDialogs?.showToast(`Attendance correction failed: ${msg}`, false);
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

	let hasModalDraftChanges = $derived.by(() => {
		if (!workbookSettings) return false;
		// Report Month is excluded from modal draft changes because it is
		// changed exclusively via the Switch Month button, not the edit dialog.
		return (
			draftSchoolId !== workbookSettings.schoolId ||
			draftSchoolName !== workbookSettings.schoolName ||
			draftSchoolYear !== workbookSettings.schoolYear ||
			draftGradeLevel !== workbookSettings.gradeLevel ||
			draftSection !== workbookSettings.section ||
			draftAdviserName !== workbookSettings.adviserName ||
			draftSchoolHeadName !== workbookSettings.schoolHeadName
		);
	});

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
			firstSchoolDay: normalizedSf2FirstSchoolDay(
				draftReportMonth,
				draftSchoolYear,
				workbookSettings.firstSchoolDay
			),
			learnerNames: []
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
			<div class="flex min-h-0 flex-col gap-5 pr-0 xl:pr-1">
				<ReportTable
					previewTemplateGradeLevel={preview.template.gradeLevel}
					previewTemplateSection={preview.template.section}
					{genderFilter}
					{matrixWeekGroups}
					{matrixStudents}
					{correctingCellKey}
					fullReview={false}
					{presentingAll}
					{hasAbsentCells}
					onToggleAttendance={toggleAttendance}
					{onPresentAll}
					onFullReviewOpen={() => (fullReviewOpen = true)}
					onGenderFilterChange={(value) => (genderFilter = value)}
				/>
			</div>

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
						<button
							type="button"
							onclick={requestExport}
							disabled={exportDisabled}
							class="control-ring inline-flex h-10 w-full items-center justify-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
						>
							<Save class="size-4" aria-hidden="true" />
							{exporting ? 'Exporting...' : 'Review Export'}
						</button>
					</div>
				</div>

				<div class="rounded-2xl border border-border bg-surface p-5">
					<div class="flex items-start justify-between gap-3">
						<div class="label-mono text-primary">Workbook identity</div>
						<button
							type="button"
							onclick={() => (workbookDetailsOpen = true)}
							disabled={!workbookSettings || savingDetails || !activeClassId}
							class="control-ring inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
							title="Edit workbook details"
						>
							<Pencil class="size-3.5" aria-hidden="true" />
							Edit
						</button>
						<button
							type="button"
							onclick={() => (monthPickerOpen = true)}
							disabled={!workbookSettings || !activeClassId}
							class="control-ring inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-background px-2.5 text-xs font-medium text-muted-foreground transition-colors hover:bg-surface hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50"
							title="Switch SF2 report month"
						>
							<Calendar class="size-3.5" aria-hidden="true" />
							Switch month
						</button>
					</div>
					<dl class="mt-4 space-y-3 text-sm">
						{@render metaRow('Class', preview.className || selectedClass?.name || 'Unlinked')}
						{@render metaRow('School ID', draftSchoolId || preview.template.schoolId || 'Blank')}
						{@render metaRow(
							'School Year',
							draftSchoolYear || preview.template.schoolYear || 'Blank'
						)}
						{@render metaRow(
							'Report Month',
							reportMonthLabel(draftReportMonth || preview.template.reportMonth)
						)}
						{@render metaRow(
							'Grade Level',
							draftGradeLevel || preview.template.gradeLevel || 'Blank'
						)}
						{@render metaRow('Section', draftSection || preview.template.section || 'Blank')}
						{@render metaRow(
							'Adviser',
							draftAdviserName || preview.template.adviserName || 'Blank'
						)}
						{@render metaRow(
							'School Head',
							draftSchoolHeadName || preview.template.schoolHeadName || 'Blank'
						)}
						{@render metaRow('Imported', formatImportedAt(preview.template.importedAt))}
					</dl>
					{#if preview.canExport !== undefined}
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
							fullReview={true}
							{fullReviewHeaderVisible}
							{workbookSettings}
							{draftSchoolId}
							{draftSchoolName}
							{draftSchoolYear}
							{draftReportMonth}
							{draftGradeLevel}
							{draftSection}
							{draftAdviserName}
							{draftSchoolHeadName}
						/>
					{/if}
					<ReportTable
						previewTemplateGradeLevel={preview.template.gradeLevel}
						previewTemplateSection={preview.template.section}
						{genderFilter}
						{matrixWeekGroups}
						{matrixStudents}
						{correctingCellKey}
						fullReview={true}
						{presentingAll}
						{hasAbsentCells}
						onToggleAttendance={toggleAttendance}
						{onPresentAll}
						onFullReviewOpen={() => (fullReviewOpen = true)}
						onGenderFilterChange={(value) => (genderFilter = value)}
					/>
				</div>
			</div>
		</div>
	</div>
{/if}

<ReportExportDialogs
	bind:this={reportDialogs}
	bind:exportDialogOpen
	bind:exportLoadingOpen
	{preview}
	{exporting}
	onConfirmExport={confirmExport}
/>

<Dialog
	open={workbookDetailsOpen}
	title="SF2 Workbook Details"
	description="Edit the header fields that are written into the SF2 workbook before export."
	maxWidth="2xl"
	onClose={() => (workbookDetailsOpen = false)}
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
				onclick={() => (workbookDetailsOpen = false)}
				class="control-ring h-10 rounded-md border border-border bg-background px-4 text-sm font-medium hover:bg-surface"
			>
				Cancel
			</button>
			<button
				type="button"
				onclick={async () => {
					const saved = await saveWorkbookDetails('SF2 workbook details saved');
					if (saved) workbookDetailsOpen = false;
				}}
				disabled={!hasModalDraftChanges || modalSaving || savingDetails}
				class="control-ring inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-4 text-sm font-semibold text-primary-foreground hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
			>
				<Save class="size-4" aria-hidden="true" />
				{modalSaving ? 'Saving...' : 'Save Details'}
			</button>
		</div>
	</div>
</Dialog>

<Dialog
	open={monthPickerOpen}
	title="Switch SF2 Report Month"
	description="Choose the school month to display. This is instant and does not rewrite the Excel workbook."
	maxWidth="sm"
	onClose={() => (monthPickerOpen = false)}
>
	<div class="grid grid-cols-2 gap-2">
		{#each SF2_SCHOOL_MONTHS as month (month.value)}
			<button
				type="button"
				onclick={async () => {
					draftReportMonth = month.value;
					monthPickerOpen = false;
					await onReportMonthChange();
				}}
				disabled={!activeClassId}
				class="control-ring flex items-center justify-between gap-2 rounded-md border px-3 py-2.5 text-left text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 {month.value ===
				(workbookSettings?.reportMonth || preview?.template?.reportMonth)
					? 'border-primary bg-primary/10 text-primary'
					: 'border-border bg-background text-foreground hover:bg-surface'}"
			>
				<span>{month.label}</span>
				{#if month.value === (workbookSettings?.reportMonth || preview?.template?.reportMonth)}
					<Check class="size-4 shrink-0" aria-hidden="true" />
				{/if}
			</button>
		{/each}
	</div>
</Dialog>

{#if monthSwitchLoading}
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Switching SF2 report month"
		class="fixed inset-0 z-[70] flex items-center justify-center bg-background/40 backdrop-blur-[2px]"
	>
		<div
			class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
			role="status"
			aria-live="polite"
		>
			<Spinner />
			<div class="space-y-1">
				<p class="text-sm font-semibold text-foreground">Switching report month…</p>
				<p class="text-xs text-muted-foreground">Updating workbook calendar and attendance marks</p>
			</div>
		</div>
	</div>
{/if}

{#if monthSwitchError}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		role="dialog"
		aria-modal="true"
		aria-label="Month switch failed"
		class="fixed inset-0 z-[70] flex items-center justify-center bg-background/40 backdrop-blur-[2px]"
		tabindex="-1"
		onkeydown={(e) => {
			if (e.key === 'Escape') monthSwitchError = null;
		}}
	>
		<div
			class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
		>
			<div class="flex size-12 items-center justify-center rounded-full bg-red-50 text-red-600">
				<CircleX class="size-6" aria-hidden="true" />
			</div>
			<div class="space-y-2">
				<h3 class="text-base font-semibold text-foreground">Could not switch month</h3>
				<p class="text-sm leading-relaxed text-muted-foreground">{monthSwitchError}</p>
			</div>
			<button
				type="button"
				onclick={() => (monthSwitchError = null)}
				class="control-ring rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent"
			>
				Dismiss
			</button>
		</div>
	</div>
{/if}

{#snippet modalTextField(label: string, value: string, field: string)}
	<label class="space-y-1.5">
		<span class="label-mono">{label}</span>
		<input
			{value}
			oninput={(event) => {
				onDraftChange(field, (event.currentTarget as HTMLInputElement).value);
			}}
			disabled={!workbookSettings || modalSaving}
			class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none disabled:opacity-60"
		/>
	</label>
{/snippet}

{#if sf2OpenStatus !== 'idle'}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		role="dialog"
		aria-modal="true"
		aria-label={sf2OpenStatus === 'error' ? 'Opening failed' : 'Opening SF2 workbook'}
		class="fixed inset-0 z-[70] flex items-center justify-center bg-background/40 backdrop-blur-[2px] transition-opacity"
		tabindex="-1"
		onkeydown={(e) => {
			if (e.key === 'Escape' && sf2OpenStatus === 'error') {
				sf2OpenStatus = 'idle';
			}
		}}
	>
		{#if sf2OpenStatus === 'error'}
			<!-- Error state -->
			<div
				class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
			>
				<div class="flex size-12 items-center justify-center rounded-full bg-red-50 text-red-600">
					<CircleX class="size-6" aria-hidden="true" />
				</div>
				<div class="space-y-2">
					<h3 class="text-base font-semibold text-foreground">Unable to open workbook</h3>
					<p class="text-sm leading-relaxed text-muted-foreground">{sf2OpenError}</p>
				</div>
				<div class="flex gap-3">
					<button
						type="button"
						onclick={() => (sf2OpenStatus = 'idle')}
						class="control-ring rounded-md border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
					>
						Close
					</button>
					<button
						type="button"
						onclick={retrySf2Open}
						class="control-ring rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground transition-colors hover:bg-accent"
					>
						Try again
					</button>
				</div>
			</div>
		{:else if sf2OpenStatus === 'success'}
			<!-- Success state -->
			<div
				class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
				in:fade={{ duration: 200 }}
			>
				<div
					class="flex size-12 items-center justify-center rounded-full bg-emerald-50 text-emerald-600"
				>
					<CheckCircle2 class="size-6" aria-hidden="true" />
				</div>
				<div class="space-y-1">
					<h3 class="text-base font-semibold text-foreground">Workbook opened!</h3>
					<p class="text-xs text-muted-foreground">
						{sf2OpenResultPath ? `Location: ${sf2OpenResultPath}` : ''}
					</p>
				</div>
			</div>
		{:else}
			<!-- Progress state (syncing) -->
			<div
				class="flex w-full max-w-sm flex-col items-center gap-5 rounded-2xl border border-border bg-surface p-8 text-center shadow-2xl"
				role="status"
				aria-live="polite"
			>
				<!-- Current friendly message -->
				<div class="space-y-1">
					<p class="text-sm font-semibold text-foreground transition-all duration-300">
						{sf2OpenDisplayMessage}
					</p>
				</div>

				<!-- Determinate progress bar with percentage -->
				<div class="w-full space-y-2">
					<div
						class="h-3 w-full overflow-hidden rounded-pill border border-primary/20 bg-background"
						role="progressbar"
						aria-valuemin="0"
						aria-valuemax={sf2OpenProgressTotal}
						aria-valuenow={sf2OpenProgressCurrent}
						aria-valuetext={`${sf2OpenProgressPercent} percent`}
					>
						<div
							class="h-full rounded-pill bg-primary transition-all duration-400 ease-out"
							style="width: {sf2OpenProgressPercent}%"
						></div>
					</div>
					<div class="label-mono text-xs text-primary">{sf2OpenProgressPercent}%</div>
				</div>

				<!-- Subtle "closing soon" hint when at 100% -->
				{#if sf2OpenProgressPercent >= 100}
					<p class="text-xs text-muted-foreground">Finalizing…</p>
				{/if}
			</div>
		{/if}
	</div>
{/if}

{#snippet metaRow(label: string, value: string)}
	<div class="flex items-center justify-between gap-3">
		<dt class="text-muted-foreground">{label}</dt>
		<dd class="font-medium">{value}</dd>
	</div>
{/snippet}
