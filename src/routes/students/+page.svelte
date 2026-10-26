<script lang="ts">
	import { onMount } from 'svelte';

	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import StudentAttendanceModal from '$lib/components/students/StudentAttendanceModal.svelte';
	import StudentList from './student-list.svelte';
	import StudentForm from './student-form.svelte';
	import StudentDeleteDialog from './student-delete-dialog.svelte';
	import { resolve } from '$app/paths';
	import { FileText, Plus } from 'lucide-svelte';
	import { studentPage } from './student-page-state.svelte';

	onMount(() => {
		void studentPage.init();
	});
</script>

<svelte:head>
	<title>Students — Attendance System</title>
	<meta name="description" content="Manage students and their attendance cards." />
</svelte:head>

<div class="flex h-full flex-col overflow-hidden">
	<PageHeader
		category="Students"
		title="Class List"
		description="Manage the student list for the assigned class."
	>
		{#snippet actions()}
			<div class="flex items-center gap-3">
				<a href={resolve('/attendance/logs')} class="btn btn-secondary">
					<FileText class="size-4" aria-hidden="true" strokeWidth={1.9} />
					View records
				</a>

				<button
					type="button"
					onclick={studentPage.openAdd}
					disabled={!studentPage.canCreateStudents}
					title={studentPage.canCreateStudents
						? 'Add student'
						: studentPage.studentCreationBlockedMessage}
					class="btn btn-primary"
				>
					<Plus class="size-4" aria-hidden="true" strokeWidth={2.2} />
					Add student
				</button>
			</div>
		{/snippet}
	</PageHeader>

	{#if studentPage.loading}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<LoadingBlock rows={4} label="Loading class list" />
		</div>
	{:else if studentPage.loadError}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<EmptyState
				tone="warning"
				title="Class list is unavailable"
				description={studentPage.loadError}
			>
				{#snippet actions()}
					<button type="button" onclick={() => void studentPage.reload()} class="btn btn-secondary">
						Retry
					</button>
				{/snippet}
			</EmptyState>
		</div>
	{:else}
		<StudentList
			students={studentPage.students}
			paginatedStudents={studentPage.paginatedStudents}
			searchTerms={studentPage.searchTerms}
			genderFilter={studentPage.genderFilter}
			sortBy={studentPage.sortBy}
			sortOrder={studentPage.sortOrder}
			currentPage={studentPage.currentPage}
			totalPages={studentPage.totalPages}
			maleStudentCount={studentPage.maleStudentCount}
			femaleStudentCount={studentPage.femaleStudentCount}
			filteredStudents={studentPage.filteredStudents}
			assignedClassLabel={studentPage.assignedClassLabel}
			canCreateStudents={studentPage.canCreateStudents}
			studentCreationBlockedMessage={studentPage.studentCreationBlockedMessage}
			onSearchChange={(value) => (studentPage.searchTerms = value)}
			onGenderFilterChange={(value) => (studentPage.genderFilter = value)}
			onToggleSort={(field) => studentPage.toggleSort(field)}
			onPageChange={(page) => studentPage.handlePageChange(page)}
			onOpenAttendance={(s) => studentPage.openAttendance(s)}
			onOpenEdit={(s) => studentPage.openEdit(s)}
			onDelete={(e, s) => studentPage.onDelete(e, s)}
			bind:availableHeight={studentPage.availableHeight}
		/>
	{/if}
</div>

<StudentAttendanceModal
	open={studentPage.attendanceModalOpen}
	student={studentPage.viewingStudent}
	onClose={() => (studentPage.attendanceModalOpen = false)}
/>

<StudentForm
	open={studentPage.dialogOpen}
	editing={studentPage.editing}
	entryMode={studentPage.entryMode}
	formName={studentPage.formName}
	formGender={studentPage.formGender}
	bulkMaleStudentNames={studentPage.bulkMaleStudentNames}
	bulkFemaleStudentNames={studentPage.bulkFemaleStudentNames}
	assignedClassLabel={studentPage.assignedClassLabel}
	canCreateStudents={studentPage.canCreateStudents}
	savingStudent={studentPage.savingStudent}
	bulkMaleNames={studentPage.bulkMaleNames}
	bulkFemaleNames={studentPage.bulkFemaleNames}
	bulkStudentCount={studentPage.bulkStudentCount}
	onClose={() => studentPage.closeDialog()}
	onSubmit={(e) => studentPage.onSubmit(e)}
	onSetEntryMode={(m) => studentPage.setEntryMode(m)}
	onFormNameChange={(value) => (studentPage.formName = value)}
	onFormGenderChange={(value) => (studentPage.formGender = value)}
	onBulkMaleChange={(value) => (studentPage.bulkMaleStudentNames = value)}
	onBulkFemaleChange={(value) => (studentPage.bulkFemaleStudentNames = value)}
/>

<StudentDeleteDialog
	deleteTarget={studentPage.deleteTarget}
	onConfirm={() => studentPage.confirmDelete()}
	onCancel={() => (studentPage.deleteTarget = null)}
/>

<FeedbackToast
	message={studentPage.toastMessage}
	onClose={() => (studentPage.toastMessage = null)}
/>
