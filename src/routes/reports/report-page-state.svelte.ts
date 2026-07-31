import { onMount, onDestroy } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';

import {
	exportSf2Workbook,
	getSf2ExportPreview,
	getSf2WorkbookSettings,
	listClasses,
	presentAllSf2PreviewAttendance,
	setSf2ReportMonth,
	syncSf2Roster,
	toggleSf2PreviewAttendance,
	updateSf2WorkbookSettings,
	type Class,
	type Sf2ExportPreview,
	type Sf2PreviewCell,
	type Sf2PreviewStudentRow,
	type Sf2WorkbookSettings
} from '$lib/db-rust';

import {
	buildMatrixWeekGroups,
	cellKey,
	errorMessage,
	formatDate,
	reportMonthLabel,
	type MatrixStudentRow
} from './report-state.svelte';

import {
	createSf2OpenState,
	cacheKey,
	getPreviewCache,
	invalidateCacheForMonth,
	invalidateAllCache
} from './report-sf2-open.svelte';

import { createWorkbookDetailsDraft } from './report-workbook-details.svelte';
import type ReportExportDialogs from './report-export-dialogs.svelte';

export function createReportPageState() {
	const sf2Open = createSf2OpenState();
	const draft = createWorkbookDetailsDraft();

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
	let workbookDetailsOpen = $state(false);
	let monthPickerOpen = $state(false);
	let monthSwitchLoading = $state(false);
	let monthSwitchError = $state<string | null>(null);
	let monthSwitchMessage = $state('');
	let modalSaving = $state(false);
	let reportDialogs = $state<ReportExportDialogs | undefined>();

	const activeClassId = $derived(
		selectedClassId || preview?.classId || preview?.template?.classId || ''
	);
	const selectedClass = $derived(classes.find((item) => item.id === activeClassId));
	const exportDisabled = $derived(
		!preview?.canExport || exporting || savingDetails || !activeClassId
	);
	const activeReportMonth = $derived(draft.reportMonth || preview?.template?.reportMonth || '');
	const matrixWeekGroups = $derived(buildMatrixWeekGroups(preview?.dates ?? [], activeReportMonth));
	const matrixStudents = $derived.by((): MatrixStudentRow[] =>
		(preview?.students ?? [])
			.filter((row) => genderFilter === 'all' || row.gender?.toLowerCase() === genderFilter)
			.map((row) => ({
				...row,
				cellsByDate: new SvelteMap(row.cells.map((cell) => [cell.date, cell]))
			}))
	);
	const hasAbsentCells = $derived((preview?.absentList.length ?? 0) > 0);
	const hasModalDraftChanges = $derived(draft.hasChanges(workbookSettings));

	onMount(() => {
		loadInitial();
	});
	onDestroy(() => {
		sf2Open.cleanup();
	});

	async function loadInitial() {
		loading = true;
		loadError = null;
		try {
			classes = await listClasses();
			const current = await getSf2ExportPreview();
			preview = current;
			selectedClassId = current.classId ?? classes[0]?.id ?? '';
			if (current.classId && current.template?.reportMonth) {
				getPreviewCache().set(cacheKey(current.classId, current.template.reportMonth), current);
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

	async function loadReport(classId?: string) {
		const cid = classId || selectedClassId || preview?.classId || preview?.template?.classId || '';
		if (!cid) return;

		const reportMonth = activeReportMonth;
		const key = cacheKey(cid, reportMonth);

		const cached = getPreviewCache().get(key);
		if (cached) {
			preview = cached;
			if (cached.classId) selectedClassId = cached.classId;
			await loadWorkbookSettings(cached.classId ?? cid);
			return;
		}

		const [nextPreview, settings] = await Promise.all([
			getSf2ExportPreview(classId),
			cid ? getSf2WorkbookSettings(cid).catch(() => null) : Promise.resolve(null)
		]);

		await new Promise((resolve) => setTimeout(resolve, 0));
		preview = nextPreview;
		if (nextPreview.classId) selectedClassId = nextPreview.classId;

		const cacheMonth = nextPreview.template?.reportMonth || reportMonth;
		if (cacheMonth) {
			getPreviewCache().set(cacheKey(cid, cacheMonth), nextPreview);
		}
		if (settings) {
			workbookSettings = settings;
			draft.hydrate(settings);
		} else {
			workbookSettings = null;
			draft.clear();
		}
	}

	async function loadWorkbookSettings(classId?: string) {
		if (!classId) {
			workbookSettings = null;
			draft.clear();
			return;
		}
		try {
			const settings = await getSf2WorkbookSettings(classId);
			workbookSettings = settings;
			draft.hydrate(settings);
		} catch {
			workbookSettings = null;
			draft.clear();
		}
	}

	async function onOpenSf2() {
		await sf2Open.open(activeClassId, preview, (msg, ok) => reportDialogs?.showToast(msg, ok));
	}

	async function retrySf2Open() {
		await sf2Open.retry(activeClassId, preview, (msg, ok) => reportDialogs?.showToast(msg, ok));
	}

	async function onPresentAll() {
		if (!activeClassId || !preview?.template || presentingAll) return;
		presentingAll = true;
		try {
			const count = await presentAllSf2PreviewAttendance(activeClassId);
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
		const missingFields = draft.blankFields();
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
		const payload = draft.buildPayload(activeClassId, workbookSettings);
		if (!payload || savingDetails) return false;
		savingDetails = true;
		modalSaving = true;
		try {
			await updateSf2WorkbookSettings(payload);
			invalidateAllCache();
			if (successMessage) reportDialogs?.showToast(successMessage);
			await loadReport(payload.classId);
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

	const MONTH_SWITCH_MESSAGES = [
		'Preparing your attendance report…',
		'Updating the workbook calendar…',
		'Applying attendance records…',
		'Almost there…',
		'Finalizing changes…'
	] as const;

	$effect(() => {
		if (monthSwitchLoading) {
			let index = -1;
			const advance = () => {
				index = (index + 1) % MONTH_SWITCH_MESSAGES.length;
				monthSwitchMessage = MONTH_SWITCH_MESSAGES[index];
			};
			advance();
			const timer = setInterval(advance, 3000);
			return () => clearInterval(timer);
		}
	});

	async function onMonthSelect(monthValue: string) {
		draft.onFieldChange('draftReportMonth', monthValue);
		monthSwitchLoading = true;
		monthPickerOpen = false;
		await onReportMonthChange();
	}

	async function onReportMonthChange() {
		const previousReportMonth =
			workbookSettings?.reportMonth || preview?.template?.reportMonth || '';
		const nextMonth = draft.reportMonth;
		if (!nextMonth || nextMonth === previousReportMonth) return;
		if (!activeClassId) {
			draft.onFieldChange('draftReportMonth', previousReportMonth);
			return;
		}

		invalidateCacheForMonth(activeClassId, previousReportMonth);
		monthSwitchLoading = true;
		monthSwitchError = null;
		await new Promise((resolve) => setTimeout(resolve, 0));

		const switchStartTime = Date.now();
		try {
			await setSf2ReportMonth(activeClassId, nextMonth);
			await loadReport(activeClassId);
			reportDialogs?.showToast(`Switched to ${reportMonthLabel(nextMonth)}`);
		} catch (error) {
			const msg = errorMessage(error, 'Failed to switch report month');
			monthSwitchError = msg;
			reportDialogs?.showToast(`Could not switch month: ${msg}`, false);
			draft.onFieldChange('draftReportMonth', previousReportMonth);
		} finally {
			const elapsed = Date.now() - switchStartTime;
			if (elapsed < 500) {
				await new Promise((resolve) => setTimeout(resolve, 500 - elapsed));
			}
			monthSwitchLoading = false;
		}
	}

	function onToggleFullReview() {
		fullReviewOpen = !fullReviewOpen;
	}

	function onWindowKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && fullReviewOpen) fullReviewOpen = false;
	}

	async function toggleAttendance(row: Sf2PreviewStudentRow, cell: Sf2PreviewCell) {
		if (!preview?.classId || !row.mapped || !cell.editable || correctingCellKey) return;
		const key = cellKey(row.studentId, cell.date);
		const markPresent = cell.status === 'absent';
		correctingCellKey = key;
		try {
			await toggleSf2PreviewAttendance(preview.classId, row.studentId, cell.date, markPresent);
			invalidateCacheForMonth(preview.classId, activeReportMonth);
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

	return {
		sf2Open,
		draft,
		get classes() {
			return classes;
		},
		set classes(v) {
			classes = v;
		},
		get selectedClassId() {
			return selectedClassId;
		},
		set selectedClassId(v) {
			selectedClassId = v;
		},
		get preview() {
			return preview;
		},
		set preview(v) {
			preview = v;
		},
		get workbookSettings() {
			return workbookSettings;
		},
		set workbookSettings(v) {
			workbookSettings = v;
		},
		get loading() {
			return loading;
		},
		set loading(v) {
			loading = v;
		},
		get loadError() {
			return loadError;
		},
		set loadError(v) {
			loadError = v;
		},
		get genderFilter() {
			return genderFilter;
		},
		set genderFilter(v) {
			genderFilter = v;
		},
		get exporting() {
			return exporting;
		},
		set exporting(v) {
			exporting = v;
		},
		get syncingRoster() {
			return syncingRoster;
		},
		set syncingRoster(v) {
			syncingRoster = v;
		},
		get presentingAll() {
			return presentingAll;
		},
		set presentingAll(v) {
			presentingAll = v;
		},
		get savingDetails() {
			return savingDetails;
		},
		set savingDetails(v) {
			savingDetails = v;
		},
		get correctingCellKey() {
			return correctingCellKey;
		},
		set correctingCellKey(v) {
			correctingCellKey = v;
		},
		get exportDialogOpen() {
			return exportDialogOpen;
		},
		set exportDialogOpen(v) {
			exportDialogOpen = v;
		},
		get exportLoadingOpen() {
			return exportLoadingOpen;
		},
		set exportLoadingOpen(v) {
			exportLoadingOpen = v;
		},
		get fullReviewOpen() {
			return fullReviewOpen;
		},
		set fullReviewOpen(v) {
			fullReviewOpen = v;
		},
		get workbookDetailsOpen() {
			return workbookDetailsOpen;
		},
		set workbookDetailsOpen(v) {
			workbookDetailsOpen = v;
		},
		get monthPickerOpen() {
			return monthPickerOpen;
		},
		set monthPickerOpen(v) {
			monthPickerOpen = v;
		},
		get monthSwitchLoading() {
			return monthSwitchLoading;
		},
		set monthSwitchLoading(v) {
			monthSwitchLoading = v;
		},
		get monthSwitchError() {
			return monthSwitchError;
		},
		set monthSwitchError(v) {
			monthSwitchError = v;
		},
		get monthSwitchMessage() {
			return monthSwitchMessage;
		},
		set monthSwitchMessage(v) {
			monthSwitchMessage = v;
		},
		get modalSaving() {
			return modalSaving;
		},
		set modalSaving(v) {
			modalSaving = v;
		},
		get reportDialogs() {
			return reportDialogs;
		},
		set reportDialogs(v) {
			reportDialogs = v;
		},
		get activeClassId() {
			return activeClassId;
		},
		get selectedClass() {
			return selectedClass;
		},
		get exportDisabled() {
			return exportDisabled;
		},
		get activeReportMonth() {
			return activeReportMonth;
		},
		get matrixWeekGroups() {
			return matrixWeekGroups;
		},
		get matrixStudents() {
			return matrixStudents;
		},
		get hasAbsentCells() {
			return hasAbsentCells;
		},
		get hasModalDraftChanges() {
			return hasModalDraftChanges;
		},
		loadInitial,
		loadReport,
		loadWorkbookSettings,
		onOpenSf2,
		retrySf2Open,
		onPresentAll,
		onSyncRoster,
		requestExport,
		confirmExport,
		saveWorkbookDetails,
		onMonthSelect,
		onReportMonthChange,
		onToggleFullReview,
		onWindowKeydown,
		toggleAttendance
	};
}
