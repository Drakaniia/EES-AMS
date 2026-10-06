<script lang="ts">
	import { commandPaletteStore } from '$lib/stores/command-palette.svelte';
	import ReportTable from './report-table.svelte';
	import ReportExportDialogs from './report-export-dialogs.svelte';
	import ReportMonthPicker from './report-month-picker.svelte';
	import ReportSf2Progress from './report-sf2-progress.svelte';
	import ReportSidebar from './ReportSidebar.svelte';
	import ReportWorkbookDetailsDialog from './ReportWorkbookDetailsDialog.svelte';
	import ReportGridSkeleton from './ReportGridSkeleton.svelte';
	import ReportLoadingStates from './ReportLoadingStates.svelte';
	import { createReportPageState } from './report-page-state.svelte';

	const page = createReportPageState();

	// Contextual palette action: switch the report month. Registered only while
	// an SF2 workbook is actually loaded.
	$effect(() => {
		if (!page.preview?.template) return;
		commandPaletteStore.register({
			id: 'reports-switch-month',
			label: 'Reports � Switch Month',
			keywords: 'change report month sf2 period',
			hint: 'Reports',
			group: 'Actions',
			run: () => {
				page.monthPickerOpen = true;
			}
		});
		return () => commandPaletteStore.unregister('reports-switch-month');
	});
</script>

<svelte:head>
	<title>Reports - Attendance System</title>
	<meta name="description" content="Review and export DepEd SF2 Excel reports." />
</svelte:head>

<svelte:window onkeydown={page.onWindowKeydown} />

<div class="flex h-full flex-col overflow-hidden">
	<!--
		Error and empty states only. This used to also own a full-page loader, which
		covered the whole route while a month or a class loaded and is why switching
		either felt like the app had hung (spec �7.4, acceptance #10). The grid now
		skeletons itself and the sidebar stays interactive.
	-->
	<ReportLoadingStates
		loading={page.loading}
		loadError={page.loadError}
		hasGrid={page.hasGrid}
		issues={page.preview?.issues ?? []}
		onRetry={page.loadInitial}
	/>

	<!--
		The layout is always up. On the very first read there is no sidebar to show
		yet, but the grid area still holds its shape rather than the page going blank
		and then jumping. A switch between months, or between classes, takes the same
		path - only the grid's cells are replaced.
	-->
	<!--
		The grid track only exists once there is something to put in it. `hasGrid` is
		false while `loading` is false exactly when there is no workbook at all, which
		is the empty state's business - rendering the skeleton there showed a second
		"loading" panel under "No SF2 workbook is ready for review".
	-->
	{#if !page.loadError && (page.loading || page.hasGrid)}
		<!-- The 320px sidebar track only exists when the sidebar is rendered.
		     Collapsing hides the sidebar, so keeping the track reserved left a dead
		     320px column of whitespace down the right of the grid. -->
		<!-- The track must be declared in BOTH states. With `grid-template-columns:
		     none` the implicit track is `auto`, i.e. sized to the grid's max-content �
		     which here is the whole 22-column table. The collapsed grid sits behind
		     three `overflow-hidden` ancestors, so that overflow was clipped and
		     unreachable rather than scrolled: the toolbar's right-hand buttons and
		     the sticky Learner column were both cut off the real screen.
		     `minmax(0,1fr)` floors the track at the container width and lets the
		     table scroll inside it. -->
		<section
			class="grid min-h-0 min-w-0 flex-1 overflow-hidden {page.sidebarCollapsed
				? 'grid-cols-[minmax(0,1fr)]'
				: 'gap-0 xl:grid-cols-[minmax(0,1fr)_320px]'}"
		>
			<div class="flex min-h-0 min-w-0 flex-col overflow-hidden">
				{#if page.loading || page.gridPending || !page.preview?.template}
					<ReportGridSkeleton label="Loading the month" />
				{:else}
					<ReportTable
						previewTemplateGradeLevel={page.preview.template.gradeLevel}
						previewTemplateSection={page.preview.template.section}
						genderFilter={page.genderFilter}
						matrixWeekGroups={page.matrixWeekGroups}
						matrixDates={page.matrixDates}
						matrixStudents={page.matrixStudents}
						correctingCellKey={page.correctingCellKey}
						sidebarCollapsed={page.sidebarCollapsed}
						presentingAll={page.presentingAll}
						hasAbsentCells={page.hasAbsentCells}
						onToggleAttendance={page.toggleAttendance}
						onPresentAll={page.onPresentAll}
						onToggleSidebar={page.onToggleSidebar}
						onGenderFilterChange={(value) => (page.genderFilter = value)}
					/>
				{/if}
			</div>

			{#if page.preview?.template && !page.loading && !page.sidebarCollapsed}
				<ReportSidebar
					preview={page.preview}
					gridPending={page.gridPending}
					classes={page.classes}
					selectedClassId={page.activeClassId}
					reportMonth={page.activeReportMonth}
					creatingMonth={page.creatingMonth}
					workbookXCount={page.monthGrid?.workbookScannedAt ? page.monthGrid.workbookXCount : null}
					workbookScannedAt={page.monthGrid?.workbookScannedAt ?? null}
					needsMonthCreate={page.needsMonthCreate}
					createMonthName={page.launch?.todayMonth ?? ''}
					selectedClass={page.selectedClass}
					draftSchoolId={page.draft.schoolId}
					draftSchoolYear={page.draft.schoolYear}
					draftReportMonth={page.draft.reportMonth}
					draftGradeLevel={page.draft.gradeLevel}
					draftSection={page.draft.section}
					draftAdviserName={page.draft.adviserName}
				draftSchoolHeadName={page.draft.schoolHeadName}
				reportYear={page.activeReportYear}
				sf2OpenStatus={page.sf2Open.status}
					workbookSettings={page.workbookSettings}
					savingDetails={page.savingDetails}
					activeClassId={page.activeClassId}
					onOpenSf2={page.onOpenSf2}
					onClassSelect={page.onClassSelect}
				onCreateMonth={() => page.onCreateMonth()}
				onEditDetails={() => (page.workbookDetailsOpen = true)}
					onSwitchMonth={() => (page.monthPickerOpen = true)}
				/>
			{/if}
		</section>
	{/if}
</div>

<ReportExportDialogs bind:this={page.reportDialogs} />

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
	currentMonth={page.activeReportMonth}
	activeClassId={page.activeClassId}
	onSelect={page.onMonthSelect}
	onClose={() => (page.monthPickerOpen = false)}
/>

<ReportSf2Progress
	status={page.sf2Open.status}
	error={page.sf2Open.error}
	resultPath={page.sf2Open.resultPath}
	displayMessage={page.sf2Open.displayMessage}
	progressPercent={page.sf2Open.progressPercent}
	showWaitHint={page.sf2Open.showWaitHint}
	onRetry={page.retrySf2Open}
	onClose={() => page.sf2Open.close()}
/>
