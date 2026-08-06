<script lang="ts">
	import { onDestroy } from 'svelte';
	import { fullPreviewStore } from '$lib/stores/full-preview.svelte';
	import ReportTable from './report-table.svelte';
	import ReportExportDialogs from './report-export-dialogs.svelte';
	import ReportMonthPicker from './report-month-picker.svelte';
	import ReportSf2Progress from './report-sf2-progress.svelte';
	import ReportSidebar from './ReportSidebar.svelte';
	import ReportWorkbookDetailsDialog from './ReportWorkbookDetailsDialog.svelte';
	import ReportMonthSwitchOverlay from './ReportMonthSwitchOverlay.svelte';
	import ReportLoadingStates from './ReportLoadingStates.svelte';
	import { createReportPageState } from './report-page-state.svelte';

	const page = createReportPageState();

	$effect(() => {
		fullPreviewStore.isActive = page.fullReviewOpen;
	});

	onDestroy(() => {
		fullPreviewStore.isActive = false;
	});
</script>

<svelte:head>
	<title>Reports - Attendance System</title>
	<meta name="description" content="Review and export DepEd SF2 Excel reports." />
</svelte:head>

<svelte:window onkeydown={page.onWindowKeydown} />

<div class="flex h-full flex-col overflow-hidden">
	<ReportLoadingStates
		loading={page.loading}
		loadError={page.loadError}
		preview={page.preview}
		onRetry={page.loadInitial}
	/>

	{#if !page.loading && !page.loadError && page.preview?.template}
		{#if page.fullReviewOpen}
			<section class="flex min-h-0 flex-1 flex-col overflow-hidden px-4 py-5 md:px-8 lg:px-10">
				<div class="flex min-h-0 flex-1 flex-col">
					<ReportTable
						previewTemplateGradeLevel={page.preview.template.gradeLevel}
						previewTemplateSection={page.preview.template.section}
						genderFilter={page.genderFilter}
						matrixWeekGroups={page.matrixWeekGroups}
						matrixStudents={page.matrixStudents}
						correctingCellKey={page.correctingCellKey}
						fullReview={true}
						presentingAll={page.presentingAll}
						hasAbsentCells={page.hasAbsentCells}
						onToggleAttendance={page.toggleAttendance}
						onPresentAll={page.onPresentAll}
						onFullReviewOpen={page.onToggleFullReview}
						onGenderFilterChange={(value) => (page.genderFilter = value)}
					/>
				</div>
			</section>
		{:else}
			<section
				class="grid min-h-0 flex-1 gap-5 overflow-hidden px-4 py-5 md:px-8 lg:px-10 xl:grid-cols-[minmax(0,1fr)_360px]"
			>
				<div class="flex min-h-0 flex-col gap-5 pr-0 xl:pr-1">
					<ReportTable
						previewTemplateGradeLevel={page.preview.template.gradeLevel}
						previewTemplateSection={page.preview.template.section}
						genderFilter={page.genderFilter}
						matrixWeekGroups={page.matrixWeekGroups}
						matrixStudents={page.matrixStudents}
						correctingCellKey={page.correctingCellKey}
						fullReview={false}
						presentingAll={page.presentingAll}
						hasAbsentCells={page.hasAbsentCells}
						onToggleAttendance={page.toggleAttendance}
						onPresentAll={page.onPresentAll}
						onFullReviewOpen={page.onToggleFullReview}
						onGenderFilterChange={(value) => (page.genderFilter = value)}
					/>
				</div>

				<ReportSidebar
					preview={page.preview}
					selectedClass={page.selectedClass}
					draftSchoolId={page.draft.schoolId}
					draftSchoolYear={page.draft.schoolYear}
					draftReportMonth={page.draft.reportMonth}
					draftGradeLevel={page.draft.gradeLevel}
					draftSection={page.draft.section}
					draftAdviserName={page.draft.adviserName}
					draftSchoolHeadName={page.draft.schoolHeadName}
					exportDisabled={page.exportDisabled}
					exporting={page.exporting}
					syncingRoster={page.syncingRoster}
					sf2OpenStatus={page.sf2Open.status}
					workbookSettings={page.workbookSettings}
					savingDetails={page.savingDetails}
					activeClassId={page.activeClassId}
					onOpenSf2={page.onOpenSf2}
					onSyncRoster={page.onSyncRoster}
					onRequestExport={page.requestExport}
					onEditDetails={() => (page.workbookDetailsOpen = true)}
					onSwitchMonth={() => (page.monthPickerOpen = true)}
				/>
			</section>
		{/if}
	{/if}
</div>

<ReportExportDialogs
	bind:this={page.reportDialogs}
	bind:exportDialogOpen={page.exportDialogOpen}
	bind:exportLoadingOpen={page.exportLoadingOpen}
	preview={page.preview}
	exporting={page.exporting}
	onConfirmExport={page.confirmExport}
/>

<ReportWorkbookDetailsDialog
	open={page.workbookDetailsOpen}
	workbookSettings={page.workbookSettings}
	draftSchoolId={page.draft.schoolId}
	draftSchoolName={page.draft.schoolName}
	draftSchoolYear={page.draft.schoolYear}
	draftReportMonth={page.draft.reportMonth}
	draftGradeLevel={page.draft.gradeLevel}
	draftSection={page.draft.section}
	draftAdviserName={page.draft.adviserName}
	draftSchoolHeadName={page.draft.schoolHeadName}
	hasModalDraftChanges={page.hasModalDraftChanges}
	modalSaving={page.modalSaving}
	savingDetails={page.savingDetails}
	onClose={() => (page.workbookDetailsOpen = false)}
	onSave={async () => {
		const saved = await page.saveWorkbookDetails('SF2 workbook details saved');
		if (saved) page.workbookDetailsOpen = false;
	}}
	onDraftChange={page.draft.onFieldChange}
/>

<ReportMonthPicker
	open={page.monthPickerOpen}
	currentMonth={page.workbookSettings?.reportMonth || page.preview?.template?.reportMonth || ''}
	activeClassId={page.activeClassId}
	onSelect={page.onMonthSelect}
	onClose={() => (page.monthPickerOpen = false)}
/>

<ReportMonthSwitchOverlay
	monthSwitchLoading={page.monthSwitchLoading}
	monthSwitchMessage={page.monthSwitchMessage}
	monthSwitchError={page.monthSwitchError}
	onDismissError={() => (page.monthSwitchError = null)}
/>

<ReportSf2Progress
	status={page.sf2Open.status}
	error={page.sf2Open.error}
	resultPath={page.sf2Open.resultPath}
	displayMessage={page.sf2Open.displayMessage}
	progressPercent={page.sf2Open.progressPercent}
	showWaitHint={page.sf2Open.showWaitHint}
	isExcelError={page.sf2Open.isExcelError}
	onRetry={page.retrySf2Open}
	onKillAndRetry={page.killAndRetrySf2Open}
	onClose={() => page.sf2Open.close()}
/>
