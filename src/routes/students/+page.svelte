<script lang="ts">
	import { onMount } from 'svelte';

	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import StudentAttendanceModal from '$lib/components/students/StudentAttendanceModal.svelte';
	import StudentCardPairDialog from '$lib/components/students/StudentCardPairDialog.svelte';
	import StudentList from './student-list.svelte';
	import StudentForm from './student-form.svelte';
	import StudentDeleteDialog from './student-delete-dialog.svelte';
	import { resolve } from '$app/paths';
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
				<a
					href={resolve('/records')}
					class="inline-flex h-10 items-center gap-2 rounded-md border border-border px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
				>
					<svg
						class="size-4"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
						<polyline points="14 2 14 8 20 8" />
						<line x1="16" y1="13" x2="8" y2="13" />
						<line x1="16" y1="17" x2="8" y2="17" />
						<polyline points="10 9 9 9 8 9" />
					</svg>
					View Records
				</a>

				<button
					type="button"
					onclick={studentPage.openAdd}
					disabled={!studentPage.canCreateStudents}
					title={studentPage.canCreateStudents ? 'Add student' : studentPage.studentCreationBlockedMessage}
					class="inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-6 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
				>
					<svg
						class="size-4"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M12 5v14M5 12h14" />
					</svg>
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
			<EmptyState tone="warning" title="Class list is unavailable" description={studentPage.loadError}>
				{#snippet actions()}
					<button
						type="button"
						onclick={() => void studentPage.reload()}
						class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
					>
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
			onOpenScan={(s) => studentPage.openScan(s)}
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
	formCardSerial={studentPage.formCardSerial}
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
	onFormCardSerialChange={(value) => (studentPage.formCardSerial = value)}
	onBulkMaleChange={(value) => (studentPage.bulkMaleStudentNames = value)}
	onBulkFemaleChange={(value) => (studentPage.bulkFemaleStudentNames = value)}
/>

<StudentCardPairDialog
	open={studentPage.scanFor !== null}
	student={studentPage.scanFor}
	bind:cardSerial={studentPage.cardSerial}
	onSave={() => studentPage.onSaveCard()}
	onClose={() => (studentPage.scanFor = null)}
/>

<StudentDeleteDialog
	deleteTarget={studentPage.deleteTarget}
	onConfirm={() => studentPage.confirmDelete()}
	onCancel={() => (studentPage.deleteTarget = null)}
/>

<FeedbackToast message={studentPage.toastMessage} onClose={() => (studentPage.toastMessage = null)} />
