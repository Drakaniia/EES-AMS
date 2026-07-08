<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import StudentAttendanceModal from '$lib/components/students/StudentAttendanceModal.svelte';
	import StudentList from './student-list.svelte';
	import StudentForm from './student-form.svelte';
	import StudentDeleteDialog from './student-delete-dialog.svelte';
	import {
		listStudents,
		saveStudent,
		createStudents,
		deleteStudent,
		listClasses,
		getSf2ExportReadiness,
		type Student,
		type StudentGender,
		type CreateStudentRequest,
		type Class,
		type Sf2ExportReadiness
	} from '$lib/db-rust';
	import { resolve } from '$app/paths';
	import {
		parseStudentNames,
		genderLabel,
		type EntryMode,
	} from './student-state.svelte';

	// ── State ────────────────────────────────────────────────────────────────
	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let sf2Readiness = $state<Sf2ExportReadiness | null>(null);
	let searchTerms = $state('');
	let genderFilter = $state<'all' | 'male' | 'female'>('all');
	let sortBy = $state<'name' | 'date'>('name');
	let sortOrder = $state<'asc' | 'desc'>('asc');
	let loading = $state(true);
	let loadError = $state<string | null>(null);
	let savingStudent = $state(false);

	let dialogOpen = $state(false);
	let attendanceModalOpen = $state(false);
	let viewingStudent = $state<Student | null>(null);
	let editing = $state<Student | null>(null);
	let scanFor = $state<Student | null>(null);

	let entryMode = $state<EntryMode>('single');
	let entryModeDirection = $state(1);
	let formName = $state('');
	let formGender = $state<StudentGender>('male');
	let formCardSerial = $state('');
	let formClassId = $state('');
	let bulkMaleStudentNames = $state('');
	let bulkFemaleStudentNames = $state('');

	let deleteTarget = $state<Student | null>(null);

	let cardSerial = $state('');
	let cardSerialInput = $state<HTMLInputElement | null>(null);

	let toastMessage = $state<string | null>(null);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// Pagination
	let currentPage = $state(1);
	let availableHeight = $state(0);
	const itemsPerPage = $derived.by(() => {
		if (availableHeight === 0) return 10;
		const rowHeight = 60;
		const headerHeight = 48;
		const verticalBuffer = 120;
		const calculated = Math.floor((availableHeight - headerHeight - verticalBuffer) / rowHeight);
		return Math.max(1, calculated);
	});

	$effect(() => {
		if (currentPage > totalPages && totalPages > 0) {
			currentPage = totalPages;
		}
	});

	$effect(() => {
		// Reset to first page when gender filter changes (results may be fewer)
		void genderFilter;
		currentPage = 1;
	});

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string) {
		toastMessage = msg;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	function setEntryMode(mode: EntryMode) {
		if (entryMode === mode) return;
		entryModeDirection = mode === 'bulk' ? 1 : -1;
		entryMode = mode;
	}

	// Computed filtered and sorted students
	const filteredStudents = $derived.by(() => {
		let result = students;

		if (searchTerms.trim()) {
			const term = searchTerms.toLowerCase();
			result = result.filter((s) => s.name.toLowerCase().includes(term));
		}

		if (genderFilter !== 'all') {
			result = result.filter((s) => s.gender === genderFilter);
		}

		result = [...result].sort((a, b) => {
			let valA: string | number = '';
			let valB: string | number = '';

			if (sortBy === 'name') {
				valA = a.name;
				valB = b.name;
			} else if (sortBy === 'date') {
				valA = a.createdAt;
				valB = b.createdAt;
			}

			if (valA < valB) return sortOrder === 'asc' ? -1 : 1;
			if (valA > valB) return sortOrder === 'asc' ? 1 : -1;
			return 0;
		});

		return result;
	});

	const totalPages = $derived(Math.ceil(filteredStudents.length / itemsPerPage));
	const paginatedStudents = $derived.by(() => {
		const start = (currentPage - 1) * itemsPerPage;
		const end = start + itemsPerPage;
		return filteredStudents.slice(start, end);
	});
	const bulkMaleNames = $derived.by(() => parseStudentNames(bulkMaleStudentNames));
	const bulkFemaleNames = $derived.by(() => parseStudentNames(bulkFemaleStudentNames));
	const bulkStudentCount = $derived(bulkMaleNames.length + bulkFemaleNames.length);
	const maleStudentCount = $derived(
		filteredStudents.filter((student) => student.gender === 'male').length
	);
	const femaleStudentCount = $derived(
		filteredStudents.filter((student) => student.gender === 'female').length
	);
	const sf2Template = $derived(sf2Readiness?.template ?? null);
	const assignedClass = $derived.by(() => {
		const sf2ClassId = sf2Template?.classId;
		return sf2ClassId ? (classes.find((classItem) => classItem.id === sf2ClassId) ?? null) : null;
	});
	const canCreateStudents = $derived(Boolean(sf2Template && assignedClass));
	const studentCreationBlockedMessage = $derived(
		sf2Template
			? 'The SF2 workbook class is unavailable. Recreate or import the SF2 workbook before adding students.'
			: 'Create an SF2 workbook before adding students.'
	);
	const assignedClassLabel = $derived(
		assignedClass?.name ?? (sf2Template ? 'Class unavailable' : 'No SF2 workbook created')
	);

	function handlePageChange(page: number) {
		currentPage = page;
	}

	function toggleSort(field: typeof sortBy) {
		if (sortBy === field) {
			sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
		} else {
			sortBy = field;
			sortOrder = 'asc';
		}
	}

	async function reload() {
		loading = true;
		loadError = null;
		try {
			const [s, c, readiness] = await Promise.all([
				listStudents(),
				listClasses(),
				getSf2ExportReadiness()
			]);
			students = s;
			classes = c;
			sf2Readiness = readiness;
			currentPage = 1;
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : 'Database error';
			loadError = msg;
			toast(`Failed to load students: ${msg}`);
		} finally {
			loading = false;
		}
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		reload();
	});

	$effect(() => {
		if (scanFor && cardSerialInput) {
			cardSerial = scanFor.cardSerial ?? '';
			cardSerialInput.focus();
		}
	});

	$effect(() => {
		if (dialogOpen && assignedClass && formClassId !== assignedClass.id) {
			formClassId = assignedClass.id;
		}
	});

	// ── Dialog helpers ───────────────────────────────────────────────────────
	function openAdd() {
		if (!canCreateStudents) {
			toast(studentCreationBlockedMessage);
			return;
		}

		editing = null;
		entryMode = 'single';
		entryModeDirection = 1;
		formName = '';
		formGender = 'male';
		formCardSerial = '';
		formClassId = assignedClass?.id ?? '';
		bulkMaleStudentNames = '';
		bulkFemaleStudentNames = '';
		dialogOpen = true;
	}

	function openEdit(s: Student) {
		editing = s;
		entryMode = 'single';
		entryModeDirection = 1;
		formName = s.name;
		formGender = s.gender ?? 'male';
		formCardSerial = s.cardSerial ?? '';
		formClassId = assignedClass?.id ?? s.classId ?? '';
		bulkMaleStudentNames = '';
		bulkFemaleStudentNames = '';
		dialogOpen = true;
	}

	function openAttendance(s: Student) {
		viewingStudent = s;
		attendanceModalOpen = true;
	}

	function closeDialog() {
		dialogOpen = false;
		editing = null;
	}

	function openScan(s: Student) {
		scanFor = s;
	}

	function createStudent(
		name: string,
		gender: StudentGender,
		classId: string,
		cardSerial?: string
	): Student {
		return {
			id: '',
			createdAt: new Date().toISOString(),
			name,
			gender,
			cardSerial: cardSerial || undefined,
			classId: classId || undefined
		};
	}

	async function onSubmit(e: SubmitEvent) {
		e.preventDefault();
		if (savingStudent) return;
		const name = formName.trim();
		const serial = formCardSerial.trim().toLowerCase();
		const classId = formClassId || assignedClass?.id || '';

		if (!editing && !canCreateStudents) {
			toast(studentCreationBlockedMessage);
			return;
		}

		if (!editing && entryMode === 'bulk') {
			if (bulkStudentCount === 0) {
				toast('Paste or type at least one student name');
				return;
			}

			try {
				savingStudent = true;
				const studentRequests: CreateStudentRequest[] = [
					...bulkMaleNames.map((bulkName) => ({
						name: bulkName,
						gender: 'male' as const,
						classId: classId || undefined
					})),
					...bulkFemaleNames.map((bulkName) => ({
						name: bulkName,
						gender: 'female' as const,
						classId: classId || undefined
					}))
				];
				const createdStudents = await createStudents(studentRequests);
				students = [...createdStudents, ...students];
				toast(`${bulkStudentCount} ${bulkStudentCount === 1 ? 'student' : 'students'} added`);
				closeDialog();
			} catch (error) {
				const msg = error instanceof Error ? error.message : 'Failed to add students';
				toast(`Error: ${msg}`);
			} finally {
				savingStudent = false;
			}
			return;
		}

		if (!name) {
			toast('Please enter a student name');
			return;
		}
		try {
			savingStudent = true;
			const studentData: Student = editing
				? {
						...editing,
						name,
						gender: formGender,
						cardSerial: serial,
						classId
					}
				: createStudent(name, formGender, classId, serial);

			const savedStudent = await saveStudent(studentData);
			students = editing
				? students.map((student) => (student.id === savedStudent.id ? savedStudent : student))
				: [savedStudent, ...students];
			toast(editing ? 'Student updated' : 'Student added');
			closeDialog();
		} catch (error) {
			console.error('Error saving student:', error);
			const msg = error instanceof Error ? error.message : 'Failed to save student';

			if (msg.includes('UNIQUE constraint failed') && msg.includes('card_serial')) {
				toast('Card serial already registered to another student.');
			} else {
				toast(`Error: ${msg}`);
			}
		} finally {
			savingStudent = false;
		}
	}

	async function confirmDelete(target = deleteTarget) {
		if (!target) return;
		await deleteStudent(target.id);
		students = students.filter((student) => student.id !== target.id);
		toast('Deleted');
		deleteTarget = null;
	}

	function onDelete(event: MouseEvent, student: Student) {
		if (event.shiftKey) {
			void confirmDelete(student);
			return;
		}
		deleteTarget = student;
	}

	async function onSaveCard() {
		const serial = cardSerial.trim().toLowerCase();
		if (!scanFor || !serial) return;
		try {
			const savedStudent = await saveStudent({ ...scanFor, cardSerial: serial });
			students = students.map((student) =>
				student.id === savedStudent.id ? savedStudent : student
			);
			toast(`Card paired to ${scanFor.name}`);
			scanFor = null;
			cardSerial = '';
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Failed to pair card';
			toast(`Card pairing failed: ${msg}`);
		}
	}
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
					onclick={openAdd}
					disabled={!canCreateStudents}
					title={canCreateStudents ? 'Add student' : studentCreationBlockedMessage}
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

	{#if loading}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<LoadingBlock rows={4} label="Loading class list" />
		</div>
	{:else if loadError}
		<div class="px-4 py-5 md:px-8 lg:px-10">
			<EmptyState tone="warning" title="Class list is unavailable" description={loadError}>
				{#snippet actions()}
					<button
						type="button"
						onclick={reload}
						class="control-ring rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium hover:bg-surface"
					>
						Retry
					</button>
				{/snippet}
			</EmptyState>
		</div>
	{:else}
		<StudentList
			{students}
			{paginatedStudents}
			{searchTerms}
			{genderFilter}
			{sortBy}
			{sortOrder}
			currentPage={currentPage}
			{totalPages}
			{maleStudentCount}
			{femaleStudentCount}
			{filteredStudents}
			{assignedClassLabel}
			{canCreateStudents}
			{studentCreationBlockedMessage}
			onSearchChange={(value) => (searchTerms = value)}
			onGenderFilterChange={(value) => (genderFilter = value)}
			onToggleSort={toggleSort}
			onPageChange={handlePageChange}
			onOpenAttendance={openAttendance}
			onOpenEdit={openEdit}
			onOpenScan={openScan}
			onDelete={onDelete}
		/>
	{/if}
</div>

<StudentAttendanceModal
	open={attendanceModalOpen}
	student={viewingStudent}
	onClose={() => (attendanceModalOpen = false)}
/>

<StudentForm
	open={dialogOpen}
	{editing}
	{entryMode}
	{entryModeDirection}
	{formName}
	{formGender}
	{formCardSerial}
	{formClassId}
	{bulkMaleStudentNames}
	{bulkFemaleStudentNames}
	{assignedClassLabel}
	{sf2Template}
	{assignedClass}
	{canCreateStudents}
	{studentCreationBlockedMessage}
	{savingStudent}
	{bulkMaleNames}
	{bulkFemaleNames}
	{bulkStudentCount}
	onClose={closeDialog}
	{onSubmit}
	onSetEntryMode={setEntryMode}
	onFormNameChange={(value) => (formName = value)}
	onFormGenderChange={(value) => (formGender = value)}
	onFormCardSerialChange={(value) => (formCardSerial = value)}
	onBulkMaleChange={(value) => (bulkMaleStudentNames = value)}
	onBulkFemaleChange={(value) => (bulkFemaleStudentNames = value)}
/>

<!-- ── Register card dialog ───────────────────────────────────────────────── -->
{#if scanFor}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (scanFor = null)}
		onkeydown={(e) => e.key === 'Escape' && (scanFor = null)}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="card-dialog-title"
	>
		<div
			class="w-full max-w-md space-y-5 rounded-2xl border border-border bg-background p-6"
		>
			<div>
				<h2 id="card-dialog-title" class="text-lg font-semibold">Pair card</h2>
				<p class="mt-1 text-sm text-muted-foreground">Enter the card serial for {scanFor.name}.</p>
			</div>

			<div class="space-y-4">
				<div class="space-y-1.5">
					<label for="manual-serial" class="label-mono">Card serial</label>
					<input
						id="manual-serial"
						bind:this={cardSerialInput}
						bind:value={cardSerial}
						placeholder="Tap card on reader or type serial…"
						autocomplete="off"
						spellcheck="false"
						class="control-ring w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm"
					/>
				</div>
			</div>

			<div class="flex justify-end gap-2">
				<button
					onclick={() => (scanFor = null)}
					class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={onSaveCard}
					disabled={!cardSerial}
					class="rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
				>
					Save
				</button>
			</div>
		</div>
	</div>
{/if}

<StudentDeleteDialog deleteTarget={deleteTarget} onConfirm={() => confirmDelete()} onCancel={() => (deleteTarget = null)} />

<FeedbackToast message={toastMessage} onClose={() => (toastMessage = null)} />
