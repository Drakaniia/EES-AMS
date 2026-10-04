<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import AttendanceGrid from './attendance-grid.svelte';
	import AttendanceLog from './attendance-log.svelte';
	import AttendanceManualLogDialog from './attendance-manual-log-dialog.svelte';
	import AttendanceDateNav from './attendance-date-nav.svelte';
	import { attendanceState } from './attendance-page-state.svelte';
	import type { Student, AttendanceType } from '$lib/api';

	onMount(() => {
		void attendanceState.init();
	});
	onDestroy(() => {
		attendanceState.destroy();
	});
</script>

<svelte:head>
	<title>Attendance - Attendance System</title>
	<meta name="description" content="Record student attendance." />
</svelte:head>

{#if attendanceState.loading || attendanceState.settingsPending}
	<div class="px-4 py-5 md:px-8 lg:px-10">
		<LoadingBlock rows={3} label="Loading attendance workspace" />
	</div>
{:else if attendanceState.loadError}
	<div class="px-4 py-5 md:px-8 lg:px-10">
		<EmptyState
			tone="warning"
			title="Attendance is unavailable"
			description={attendanceState.loadError}
		>
			{#snippet actions()}
				<button
					type="button"
					onclick={() => void attendanceState.loadInitial()}
					class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
				>
					Retry
				</button>
			{/snippet}
		</EmptyState>
	</div>
{:else}
	<div class="flex min-h-0 flex-1 flex-col px-4 py-5 md:px-8 lg:px-10">
		<AttendanceGrid
			manualStudents={attendanceState.manualStudents}
			bind:manualViewMode={attendanceState.manualViewMode}
			isProcessing={attendanceState.isProcessing}
			dateLoading={attendanceState.dateLoading}
			presentCount={attendanceState.presentCount}
			absentCount={attendanceState.absentCount}
			pendingCount={attendanceState.pendingCount}
			rosterCount={attendanceState.selectedClassRosterCount}
			rosterQuery={attendanceState.rosterQuery}
			isScheduledDayValue={attendanceState.isScheduledDayValue}
			isPresentingAll={attendanceState.isPresentingAll}
			onMarkStudent={(student: Student, action: AttendanceType | null) =>
				void attendanceState.markStudent(student, action)}
			onPresentAllStudents={() => void attendanceState.presentAllStudents()}
			onClearAllAttendance={() => void attendanceState.clearAllAttendance()}
			onRosterQueryChange={(value) => (attendanceState.rosterQuery = value)}
			onGetNextAttendanceType={(student) => attendanceState.getNextAttendanceType(student)}
			onGetStudentStatus={(student) => attendanceState.getStudentStatus(student)}
			onMarkAbsent={(student: Student) => void attendanceState.markAbsent(student)}
		>
			{#snippet dateNav()}
				<AttendanceDateNav
					selectedDate={attendanceState.selectedDate}
					dateLoading={attendanceState.dateLoading}
					isProcessing={attendanceState.isProcessing}
					displayDateLabel={attendanceState.displayDateLabel}
					onDateOffset={(offset) => attendanceState.handleDateOffset(offset)}
					onDateSelect={(date) => void attendanceState.selectAttendanceDate(date)}
				/>
			{/snippet}
		</AttendanceGrid>
	</div>
{/if}

<AttendanceManualLogDialog
	open={attendanceState.pickerOpen}
	bind:pickerQuery={attendanceState.pickerQuery}
	pickerStudents={attendanceState.pickerStudents}
	selectedDateLabel={attendanceState.selectedDateLabel}
	selectedClassId={attendanceState.selectedClassId}
	isProcessing={attendanceState.isProcessing}
	dateLoading={attendanceState.dateLoading}
	getNextAttendanceType={(student) => attendanceState.getNextAttendanceType(student)}
	getStudentStatus={(student) => attendanceState.getStudentStatus(student)}
	markStudent={(student: Student, action: AttendanceType | null, closePicker = false) =>
		attendanceState.markStudent(student, action, closePicker)}
	onMarkAbsent={(student: Student) => void attendanceState.markAbsent(student)}
/>

<AttendanceLog
	bind:this={attendanceState.attendanceLog}
	bind:log={attendanceState.log}
	onUndo={(id) => attendanceState.handleUndo(id)}
/>
