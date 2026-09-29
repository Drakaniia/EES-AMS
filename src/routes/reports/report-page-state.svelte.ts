import { onMount, onDestroy } from 'svelte';

import {
	createSf2MonthFile,
	exportSf2Workbook,
	getSf2LaunchMonth,
	getSf2MonthPreview,
	getSf2WorkbookSettings,
	listClasses,
	presentAllSf2PreviewAttendance,
	syncSf2Roster,
	toggleSf2PreviewAttendance,
	updateSf2WorkbookSettings,
	type Class,
	type Sf2ExportPreview,
	type Sf2LaunchMonth,
	type Sf2MonthGridPreview,
	type Sf2PreviewCell,
	type Sf2PreviewStudentRow,
	type Sf2WorkbookSettings
} from '$lib/db-rust';

import {
	buildMatrixRows,
	buildMatrixWeekGroups,
	cellKey,
	errorMessage,
	flattenMatrixSlots,
	formatDate,
	monthGridToPreview,
	reportMonthLabel
} from './report-state.svelte';

import {
	createSf2OpenState,
	getMonthCache,
	invalidateAllMonthCache,
	invalidateClassMonths,
	invalidateMonthCache,
	monthCacheKey
} from './report-sf2-open.svelte';

import { createWorkbookDetailsDraft } from './report-workbook-details.svelte';
import type ReportExportDialogs from './report-export-dialogs.svelte';

/**
 * Reports page state.
 *
 * The month switch is the whole point of this file. Switching a month is:
 *
 * ```text
 * reportMonth = 'OCTOBER'
 *   └─ $derived → matrixWeekGroups / matrixStudents recomputed from monthGrid
 *   └─ one cached getSf2MonthPreview('OCTOBER')
 * ```
 *
 * There is no `setSf2ReportMonth`, no Excel, no mutation, no progress listener
 * and no modal. What used to be a full COM session - renaming eleven hidden
 * tabs, re-analysing the workbook, rewriting COUNTIF and summary formulas, all
 * behind a `ReportMonthSwitchOverlay` - is one read-only SQL query whose result
 * is cached per `${classId}:${schoolYear}:${month}`.
 */
export function createReportPageState() {
	const sf2Open = createSf2OpenState();
	const draft = createWorkbookDetailsDraft();

	let classes = $state<Class[]>([]);
	let selectedClassId = $state('');
	// `$state.raw` on purpose: the preview is replaced wholesale on every load and
	// never mutated in place, so deep proxying its thousands of cell objects only
	// adds per-property reactive reads to the grid's hot render path.
	let preview = $state.raw<Sf2ExportPreview | null>(null);
	// The month read, kept alongside the projected preview so the page knows which
	// month the grid belongs to and which year it is in. `reportYear` comes from
	// the database, never from the browser's clock - see `report-state.svelte.ts`.
	let monthGrid = $state.raw<Sf2MonthGridPreview | null>(null);
	let launch = $state.raw<Sf2LaunchMonth | null>(null);
	let reportMonth = $state('');
	let schoolYear = $state('');
	let workbookSettings = $state<Sf2WorkbookSettings | null>(null);
	let loading = $state(true);
	let loadError = $state<string | null>(null);
	// The grid's own loading flag. Distinct from `loading` because a month or
	// class switch must not blank the page: the sidebar, the month picker and the
	// class selector all stay usable while this is true.
	let gridPending = $state(false);
	let creatingMonth = $state(false);
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
	let modalSaving = $state(false);
	let reportDialogs = $state<ReportExportDialogs | undefined>();

	const activeClassId = $derived(
		selectedClassId || monthGrid?.classId || preview?.classId || preview?.template?.classId || ''
	);
	const selectedClass = $derived(classes.find((item) => item.id === activeClassId));
	const exportDisabled = $derived(
		!preview?.canExport || exporting || savingDetails || !activeClassId
	);
	const activeReportMonth = $derived(
		reportMonth || draft.reportMonth || preview?.template?.reportMonth || ''
	);
	/**
	 * The month the grid is drawing, and the year it is drawing it in.
	 *
	 * `reportYear` is the read month's own year. `buildMatrixWeekGroups` falls
	 * back to the first mapped date and then to the clock only when there is no
	 * read at all, which is the skeleton case and not a month anyone will mistake
	 * for a real one.
	 */
	const activeReportYear = $derived(monthGrid?.reportYear);
	const matrixWeekGroups = $derived(
		buildMatrixWeekGroups(preview?.dates ?? [], activeReportMonth, activeReportYear)
	);
	const matrixDates = $derived(flattenMatrixSlots(matrixWeekGroups));
	// Built once per preview load / gender filter change: rows arrive with their
	// cells already projected onto the visible date columns.
	const matrixStudents = $derived(
		buildMatrixRows(preview?.students ?? [], matrixDates, genderFilter)
	);
	const hasAbsentCells = $derived((preview?.absentList.length ?? 0) > 0);
	const hasModalDraftChanges = $derived(draft.hasChanges(workbookSettings));
	/** The grid has something to draw: a read landed, or a legacy preview did. */
	const hasGrid = $derived(monthGrid !== null || preview !== null);
	/** Is the app showing a month other than the one it was opened in? */
	const needsMonthCreate = $derived(!!launch && launch.fellBack && launch.todayCanCreate);

	onMount(() => {
		loadInitial();
	});
	onDestroy(() => {
		sf2Open.cleanup();
	});

	/**
	 * First load: the class, the month to open (D5), then that month's grid.
	 *
	 * The launch month is resolved by the backend so the D5 fallback and the E1
	 * warning are decided in one place, from the same rows the month read will
	 * use. Two reads, no Excel.
	 */
	async function loadInitial() {
		loading = true;
		loadError = null;
		try {
			classes = await listClasses();
			const classId = classes[0]?.id ?? '';
			selectedClassId = classId;

			if (!classId) {
				return;
			}

			launch = await getSf2LaunchMonth(classId);
			reportMonth = launch.month;
			schoolYear = launch.schoolYear;
			announceLaunch();

			await loadMonth(classId, launch.schoolYear, launch.month);
			await loadWorkbookSettings(classId);
		} catch (error) {
			const msg = errorMessage(error, 'Failed to load reports');
			loadError = msg;
			reportDialogs?.showToast(`Reports failed: ${msg}`, false);
		} finally {
			loading = false;
		}
	}

	/**
	 * Say out loud when the app is not showing today's month.
	 *
	 * Edge case E1, in the spec's own words: *"Showing <MONTH>. Create
	 * <TODAY'S MONTH> to switch."* The offer to create is a button in the
	 * sidebar rather than an action on the toast, because the toast is gone
	 * before anyone has finished reading it and the sidebar is where they will be
	 * looking anyway.
	 */
	function announceLaunch() {
		if (!launch) return;
		if (launch.fellBack) {
			reportDialogs?.showToast(
				`Showing ${reportMonthLabel(launch.month)}. ` +
					(launch.todayCanCreate
						? `Create ${reportMonthLabel(launch.todayMonth)} to switch.`
						: `${reportMonthLabel(launch.todayMonth)} has no SF2 workbook yet.`)
			);
		}
		for (const issue of launch.issues) {
			reportDialogs?.showToast(issue, false);
		}
	}

	/**
	 * Read one month, from the cache when it is already there.
	 *
	 * This is the only thing a month switch calls. It is `await`ed so the caller
	 * can report a failure, but the grid is not blocked on it: `reportMonth` is
	 * set first and the `$derived` chain repaints from whatever is on screen while
	 * the read is in flight.
	 */
	async function loadMonth(classId: string, year: string, month: string) {
		if (!classId || !month) return;
		const cache = getMonthCache();
		const key = monthCacheKey(classId, year, month);
		const cached = cache.get(key);
		if (cached) {
			applyMonth(cached);
			return;
		}
		const grid = await getSf2MonthPreview(month, classId, year);
		cache.set(key, grid);
		applyMonth(grid);
	}

	function applyMonth(grid: Sf2MonthGridPreview) {
		monthGrid = grid;
		reportMonth = grid.month;
		// The read reports the school year it actually resolved. Taking it from the
		// read rather than keeping whatever the previous class used is what stops
		// a class switch from looking in the wrong year.
		schoolYear = grid.schoolYear || schoolYear;
		preview = monthGridToPreview(grid);
		if (grid.classId) selectedClassId = grid.classId;
	}

	/**
	 * Switch month. Instant, and nothing else.
	 *
	 * The selected month is applied to state *before* the read is awaited, so the
	 * grid's weekday headers change on the same frame as the click. If the read
	 * then fails the selection is rolled back, because a header claiming one month
	 * over another month's marks is worse than a failed switch.
	 */
	async function onMonthSelect(monthValue: string) {
		monthPickerOpen = false;
		await switchToMonth(monthValue);
	}

	async function switchToMonth(monthValue: string) {
		const classId = activeClassId;
		if (!classId || !monthValue) return;
		const previousMonth = activeReportMonth;
		if (monthValue === previousMonth && monthGrid) return;

		// The instant part: state changes now, the grid repaints from it, and the
		// read happens underneath.
		reportMonth = monthValue;
		draft.onFieldChange('draftReportMonth', monthValue);
		gridPending = true;
		try {
			await loadMonth(classId, schoolYear, monthValue);
			reportDialogs?.showToast(`Switched to ${reportMonthLabel(monthValue)}`);
		} catch (error) {
			const msg = errorMessage(error, 'Failed to load the month');
			reportMonth = previousMonth;
			draft.onFieldChange('draftReportMonth', previousMonth);
			reportDialogs?.showToast(`Could not switch month: ${msg}`, false);
		} finally {
			gridPending = false;
		}
	}

	/**
	 * Switch class. Also instant, and also a read (spec §7.4).
	 *
	 * The sidebar stays interactive throughout: `gridPending` only gates the grid,
	 * so the class selector, the month picker and the workbook identity panel all
	 * keep responding while the new class's month is being read.
	 */
	async function onClassSelect(classId: string) {
		if (!classId || classId === activeClassId) return;
		selectedClassId = classId;
		gridPending = true;
		try {
			// An empty school year, so the read resolves the new class's own year
			// rather than reusing the previous class's.
			await loadMonth(classId, '', reportMonth);
			await loadWorkbookSettings(classId);
		} catch (error) {
			const msg = errorMessage(error, 'Failed to load the class');
			reportDialogs?.showToast(`Could not switch class: ${msg}`, false);
		} finally {
			gridPending = false;
		}
	}

	/**
	 * The one-click create of edge case E1.
	 *
	 * Offered only for a month that has school days and no file. It is a separate
	 * command from the read, deliberately: a switch never writes, and this is not
	 * a switch.
	 */
	async function onCreateMonth(monthValue?: string) {
		const classId = activeClassId;
		const month = monthValue || launch?.todayMonth || launch?.month || '';
		if (!classId || !month || creatingMonth) return;
		creatingMonth = true;
		try {
			await createSf2MonthFile(month, classId);
			invalidateMonthCache(classId, schoolYear, month);
			reportDialogs?.showToast(`Created ${reportMonthLabel(month)}.`);
			await switchToMonth(month);
			if (launch) launch = { ...launch, todayCanCreate: false };
		} catch (error) {
			const msg = errorMessage(error, 'Could not create the month');
			reportDialogs?.showToast(`Could not create ${reportMonthLabel(month)}: ${msg}`, false);
		} finally {
			creatingMonth = false;
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

	async function killAndRetrySf2Open() {
		await sf2Open.killAndRetry(activeClassId, preview, (msg, ok) =>
			reportDialogs?.showToast(msg, ok)
		);
	}

	/** Re-read the month on screen, dropping its cached copy first. */
	async function refreshCurrentMonth() {
		if (!activeClassId || !activeReportMonth) return;
		invalidateMonthCache(activeClassId, schoolYear, activeReportMonth);
		gridPending = true;
		try {
			await loadMonth(activeClassId, schoolYear, activeReportMonth);
		} catch (error) {
			const msg = errorMessage(error, 'Failed to refresh');
			reportDialogs?.showToast(`Could not refresh: ${msg}`, false);
		} finally {
			gridPending = false;
		}
	}

	async function onPresentAll() {
		if (!activeClassId || !preview?.template || presentingAll) return;
		presentingAll = true;
		try {
			const count = await presentAllSf2PreviewAttendance(activeClassId);
			invalidateClassMonths(activeClassId);
			reportDialogs?.showToast(`All students cleared to Present (${count} marks cleared)`);
			await refreshCurrentMonth();
		} catch (error) {
			const msg = errorMessage(error, 'Present All failed');
			reportDialogs?.showToast(`Could not mark all present: ${msg}`, false);
		} finally {
			presentingAll = false;
		}
	}

	/**
	 * Pull the "X" marks back out of the SF2 working workbook and record them as
	 * absences.
	 *
	 * The workbook is the school's official record, so it is the only surviving
	 * copy of a day's marks when the app's database has been rebuilt. That is
	 * exactly what the startup self-heal now does, unattended, at every launch
	 * (spec D6, §8.2, acceptance #15) — additively, with nothing to click. The
	 * button that used to live here is gone (acceptance #14): a manual repair for
	 * something that has already run by the time the page is on screen is a
	 * control that only works by accident.
	 *
	 * The service behind it, `attendance_import::import_absent_marks_from_workbook`,
	 * stays reachable from Rust.
	 */

	async function onSyncRoster() {
		if (!activeClassId || !preview?.template || syncingRoster) return;
		syncingRoster = true;
		try {
			await syncSf2Roster(activeClassId);
			invalidateAllMonthCache();
			reportDialogs?.showToast('Roster synced! All students mapped to SF2 workbook.');
			await refreshCurrentMonth();
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
			invalidateMonthCache(activeClassId, schoolYear, activeReportMonth);
			reportDialogs?.showToast(`SF2 exported and opened: ${result.outputPath}`);
			await refreshCurrentMonth();
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
			invalidateAllMonthCache();
			if (successMessage) reportDialogs?.showToast(successMessage);
			await refreshCurrentMonth();
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
			invalidateMonthCache(activeClassId, schoolYear, activeReportMonth);
			await loadMonth(activeClassId, schoolYear, activeReportMonth);
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
		get monthGrid() {
			return monthGrid;
		},
		get launch() {
			return launch;
		},
		get reportMonth() {
			return reportMonth;
		},
		set reportMonth(v) {
			reportMonth = v;
		},
		get schoolYear() {
			return schoolYear;
		},
		set schoolYear(v) {
			schoolYear = v;
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
		get gridPending() {
			return gridPending;
		},
		set gridPending(v) {
			gridPending = v;
		},
		get creatingMonth() {
			return creatingMonth;
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
		get activeReportYear() {
			return activeReportYear;
		},
		get matrixWeekGroups() {
			return matrixWeekGroups;
		},
		get matrixDates() {
			return matrixDates;
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
		get hasGrid() {
			return hasGrid;
		},
		get needsMonthCreate() {
			return needsMonthCreate;
		},
		loadInitial,
		loadMonth,
		loadWorkbookSettings,
		onOpenSf2,
		retrySf2Open,
		killAndRetrySf2Open,
		onPresentAll,
		onSyncRoster,
		onCreateMonth,
		onMonthSelect,
		onClassSelect,
		refreshCurrentMonth,
		requestExport,
		confirmExport,
		saveWorkbookDetails,
		onToggleFullReview,
		onWindowKeydown,
		toggleAttendance
	};
}
