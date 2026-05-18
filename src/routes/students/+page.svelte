<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import StudentAttendanceModal from '$lib/components/students/StudentAttendanceModal.svelte';
	import {
		listStudents,
		saveStudent,
		deleteStudent,
		listClasses,
		type Student,
		type Class
	} from '$lib/db-rust';
	import { resolve } from '$app/paths';

	// ── State ────────────────────────────────────────────────────────────────
	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let selectedClassId = $state<string>('');
	let searchTerms = $state('');
	let sortBy = $state<'name' | 'number' | 'date'>('name');
	let sortOrder = $state<'asc' | 'desc'>('asc');

	let dialogOpen = $state(false);
	let attendanceModalOpen = $state(false);
	let viewingStudent = $state<Student | null>(null);
	let editing = $state<Student | null>(null);
	let scanFor = $state<Student | null>(null);

	let entryMode = $state<'single' | 'bulk'>('single');
	let formName = $state('');
	let formStudentNumber = $state('');
	let formCardSerial = $state('');
	let formClassId = $state('');
	let bulkStudentNames = $state('');

	let deleteTarget = $state<Student | null>(null);

	let cardSerial = $state('');

	let toastMessage = $state<string | null>(null);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// Pagination
	let currentPage = $state(1);
	let availableHeight = $state(0);
	const itemsPerPage = $derived.by(() => {
		if (availableHeight === 0) return 10;
		// availableHeight is bound to the <section> which has pb-20 (80px).
		// The table container has mt-8 (32px).
		// We need a conservative buffer to ensure no row is partially covered.
		const rowHeight = 60; // Safer estimate for row height including borders
		const headerHeight = 48; // Table header height
		const verticalBuffer = 120; // Accounts for mt-8 (32px), pb-20 (80px), and extra safety
		const calculated = Math.floor((availableHeight - headerHeight - verticalBuffer) / rowHeight);
		return Math.max(1, calculated);
	});

	$effect(() => {
		if (currentPage > totalPages && totalPages > 0) {
			currentPage = totalPages;
		}
	});

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string) {
		toastMessage = msg;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	// Computed filtered and sorted students
	const filteredStudents = $derived.by(() => {
		let result = students;

		// Search
		if (searchTerms.trim()) {
			const term = searchTerms.toLowerCase();
			result = result.filter(
				(s) => s.name.toLowerCase().includes(term) || s.studentNumber.toLowerCase().includes(term)
			);
		}

		// Sort
		result = [...result].sort((a, b) => {
			let valA: string | number = '';
			let valB: string | number = '';

			if (sortBy === 'name') {
				valA = a.name;
				valB = b.name;
			} else if (sortBy === 'number') {
				valA = a.studentNumber;
				valB = b.studentNumber;
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

	// Computed pagination values
	const totalPages = $derived(Math.ceil(filteredStudents.length / itemsPerPage));
	const paginatedStudents = $derived.by(() => {
		const start = (currentPage - 1) * itemsPerPage;
		const end = start + itemsPerPage;
		return filteredStudents.slice(start, end);
	});
	const bulkNames = $derived.by(() =>
		bulkStudentNames
			.split(/\r?\n/)
			.map((name) => name.trim())
			.filter(Boolean)
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
		try {
			const [s, c] = await Promise.all([listStudents(selectedClassId || undefined), listClasses()]);
			students = s;
			classes = c;
			currentPage = 1;
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : 'Database error';
			toast(`Failed to load students: ${msg}`);
		}
	}

	async function exportStudents() {
		// Simple CSV export for students
		const headers = ['Name', 'Student Number', 'Class', 'Card Serial', 'Created At'];
		const rows = students.map((s) => [
			s.name,
			displayStudentNumber(s.studentNumber),
			getClassName(s.classId),
			s.cardSerial || '',
			s.createdAt
		]);

		const csvContent =
			'data:text/csv;charset=utf-8,' +
			[headers.join(','), ...rows.map((r) => r.map((cell) => `"${cell}"`).join(','))].join('\n');

		const encodedUri = encodeURI(csvContent);
		const link = document.createElement('a');
		link.setAttribute('href', encodedUri);
		link.setAttribute('download', 'class_list.csv');
		document.body.appendChild(link);
		link.click();
		document.body.removeChild(link);
		toast('Student list exported');
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		reload();
	});

	// Re-load when filter changes
	$effect(() => {
		if (selectedClassId !== undefined) {
			reload();
		}
	});

	// ── Dialog helpers ───────────────────────────────────────────────────────
	function openAdd() {
		editing = null;
		entryMode = 'single';
		formName = '';
		formStudentNumber = '';
		formCardSerial = '';
		formClassId = selectedClassId || (classes.length > 0 ? classes[0].id : '');
		bulkStudentNames = '';
		dialogOpen = true;
	}

	function openEdit(s: Student) {
		editing = s;
		entryMode = 'single';
		formName = s.name;
		formStudentNumber = isGeneratedStudentNumber(s.studentNumber) ? '' : s.studentNumber;
		formCardSerial = s.cardSerial ?? '';
		formClassId = s.classId ?? '';
		bulkStudentNames = '';
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

	function isGeneratedStudentNumber(value: string) {
		return value.startsWith('temp-');
	}

	function displayStudentNumber(value: string) {
		return isGeneratedStudentNumber(value) ? '—' : value;
	}

	function makeStudentNumber(value: string) {
		return value.trim() || `temp-${crypto.randomUUID()}`;
	}

	function createStudent(
		name: string,
		studentNumber: string,
		classId: string,
		cardSerial?: string
	): Student {
		return {
			id: '',
			createdAt: new Date().toISOString(),
			name,
			studentNumber: makeStudentNumber(studentNumber),
			cardSerial: cardSerial || undefined,
			classId: classId || undefined
		};
	}

	async function onSubmit(e: SubmitEvent) {
		e.preventDefault();
		const name = formName.trim();
		const num = formStudentNumber.trim();
		const serial = formCardSerial.trim().toLowerCase();
		const classId = formClassId;

		if (!editing && entryMode === 'bulk') {
			if (bulkNames.length === 0) {
				toast('Paste or type at least one student name');
				return;
			}

			try {
				for (const bulkName of bulkNames) {
					await saveStudent(createStudent(bulkName, '', classId));
				}
				toast(`${bulkNames.length} ${bulkNames.length === 1 ? 'student' : 'students'} added`);
				closeDialog();
				reload();
			} catch (error) {
				const msg = error instanceof Error ? error.message : 'Failed to add students';
				toast(`Error: ${msg}`);
			}
			return;
		}

		if (!name) {
			toast('Please enter a student name');
			return;
		}

		// Check for duplicate student number only when one is provided.
		if (num && !editing) {
			const existingStudent = students.find((s) => s.studentNumber === num);
			if (existingStudent) {
				toast(
					`Student number "${num}" already exists for ${existingStudent.name}. Please use a different number.`
				);
				return;
			}
		}

		try {
			const studentData: Student = editing
				? {
						...editing,
						name,
						studentNumber: makeStudentNumber(num),
						cardSerial: serial || undefined,
						classId: classId || undefined
					}
				: createStudent(name, num, classId, serial);

			await saveStudent(studentData);
			toast(editing ? 'Student updated' : 'Student added');
			closeDialog();
			reload();
		} catch (error) {
			console.error('Error saving student:', error);
			const msg = error instanceof Error ? error.message : 'Failed to save student';

			// Check for common database errors
			if (msg.includes('UNIQUE constraint failed') && msg.includes('student_number')) {
				toast('Student number already exists. Please use a different student number.');
			} else if (msg.includes('UNIQUE constraint failed') && msg.includes('card_serial')) {
				toast('Card serial already registered to another student.');
			} else {
				toast(`Error: ${msg}`);
			}
		}
	}

	async function confirmDelete(target = deleteTarget) {
		if (!target) return;
		await deleteStudent(target.id);
		toast('Deleted');
		deleteTarget = null;
		reload();
	}

	async function onDelete(event: MouseEvent, student: Student) {
		if (event.shiftKey) {
			await confirmDelete(student);
			return;
		}

		deleteTarget = student;
	}

	async function onSaveCard() {
		if (!scanFor || !cardSerial) return;
		await saveStudent({ ...scanFor, cardSerial: cardSerial.toLowerCase() });
		toast(`Card paired to ${scanFor.name}`);
		scanFor = null;
		reload();
	}

	function getClassName(id?: string) {
		if (!id) return '—';
		return classes.find((c) => c.id === id)?.name ?? 'Unknown';
	}
</script>

<svelte:head>
	<title>Students — Attendance System</title>
	<meta name="description" content="Manage students and their ID cards." />
</svelte:head>

<AppShell>
	<div class="flex h-full flex-col overflow-hidden">
		<PageHeader
			category="Students"
			title="Class List"
			description="Manage your student list and their identification cards."
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
						onclick={exportStudents}
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
							<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
							<polyline points="7 10 12 15 17 10" />
							<line x1="12" y1="15" x2="12" y2="3" />
						</svg>
						Export CSV
					</button>

					<button
						onclick={openAdd}
						class="inline-flex h-10 items-center gap-2 rounded-pill bg-primary px-6 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
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

		<!-- Tools Bar -->
		<section class="grid gap-4 px-6 pt-8 md:grid-cols-2 md:px-12 lg:grid-cols-3">
			<!-- Search -->
			<div class="space-y-2">
				<div class="label-mono">Search Students</div>
				<div class="relative">
					<svg
						class="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<circle cx="11" cy="11" r="8" />
						<path d="m21 21-4.3-4.3" />
					</svg>
					<input
						type="text"
						bind:value={searchTerms}
						placeholder="Name or student number…"
						class="h-10 w-full rounded-md border border-border bg-background pr-4 pl-10 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</div>
			</div>

			<!-- Class Filter -->
			<div class="space-y-2">
				<div class="label-mono">Filter by Class</div>
				<select
					bind:value={selectedClassId}
					class="h-10 w-full rounded-md border border-border bg-background px-3 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				>
					<option value="">All Classes</option>
					{#each classes as c (c.id)}
						<option value={c.id}>{c.name}</option>
					{/each}
				</select>
			</div>

			<!-- Stats -->
			<div class="space-y-2">
				<div class="label-mono">Total Students</div>
				<div class="flex h-10 items-center font-mono text-lg font-bold">
					{filteredStudents.length}
					<span class="ml-2 text-xs font-normal text-muted-foreground">
						(out of {students.length})
					</span>
				</div>
			</div>
		</section>

		<!-- Class List -->
		<section class="min-h-0 flex-1 px-6 pb-20 md:px-12" bind:clientHeight={availableHeight}>
			{#if students.length === 0}
				{@render emptyState()}
			{:else}
				<div class="mt-8 overflow-hidden rounded-2xl border border-border bg-card">
					<table class="w-full text-sm">
						<thead class="bg-surface text-left">
							<tr>
								<th class="label-mono px-4 py-3">
									<button
										onclick={() => toggleSort('name')}
										class="inline-flex items-center gap-1 transition-colors hover:text-primary"
									>
										Name
										{#if sortBy === 'name'}
											<svg
												class="size-3"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2"
											>
												<path d={sortOrder === 'asc' ? 'm18 15-6-6-6 6' : 'm6 9 6 6 6-6'} />
											</svg>
										{/if}
									</button>
								</th>
								<th class="label-mono px-4 py-3">
									<button
										onclick={() => toggleSort('number')}
										class="inline-flex items-center gap-1 transition-colors hover:text-primary"
									>
										Student #
										{#if sortBy === 'number'}
											<svg
												class="size-3"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2"
											>
												<path d={sortOrder === 'asc' ? 'm18 15-6-6-6 6' : 'm6 9 6 6 6-6'} />
											</svg>
										{/if}
									</button>
								</th>
								<th class="label-mono px-4 py-3">Class</th>
								<th class="label-mono px-4 py-3">Card</th>
								<th class="label-mono w-36 px-4 py-3 text-right">Actions</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-border">
							{#each paginatedStudents as s (s.id)}
								<tr>
									<td class="px-4 py-3">
										<button
											onclick={() => openAttendance(s)}
											class="group flex items-center gap-2 text-left font-medium transition-colors hover:text-primary"
										>
											{s.name}
											<svg
												class="size-3 opacity-0 transition-opacity group-hover:opacity-100"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2.5"
												stroke-linecap="round"
												stroke-linejoin="round"
											>
												<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
												<polyline points="15 3 21 3 21 9" />
												<line x1="10" y1="14" x2="21" y2="3" />
											</svg>
										</button>
									</td>
									{@render td(displayStudentNumber(s.studentNumber), 'font-mono')}
									<td class="px-4 py-3">
										<span class="rounded-pill border border-border bg-surface px-2 py-0.5 text-xs">
											{getClassName(s.classId)}
										</span>
									</td>
									<td class="px-4 py-3 font-mono text-xs">
										{#if s.cardSerial}
											<span class="rounded-pill border border-border bg-surface px-2 py-1"
												>{s.cardSerial}</span
											>
										{:else}
											<span class="text-muted-foreground">—</span>
										{/if}
									</td>
									<td class="px-4 py-3 text-right">
										<div class="inline-flex gap-1">
											<!-- View Records -->
											<a
												href={resolve(`/records?studentId=${s.id}`)}
												class="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface"
												title="View attendance records"
											>
												<svg
													class="size-3.5"
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
											</a>
											<!-- Pair card -->
											<button
												onclick={() => (scanFor = s)}
												class="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface"
												title="Pair card"
											>
												<svg
													class="size-3.5"
													viewBox="0 0 24 24"
													fill="none"
													stroke="currentColor"
													stroke-width="2"
													stroke-linecap="round"
													stroke-linejoin="round"
												>
													<rect x="2" y="5" width="20" height="14" rx="2" />
													<path d="M2 10h20" />
												</svg>
											</button>
											<!-- Edit -->
											<button
												onclick={() => openEdit(s)}
												class="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface"
												title="Edit student"
											>
												<svg
													class="size-3.5"
													viewBox="0 0 24 24"
													fill="none"
													stroke="currentColor"
													stroke-width="2"
													stroke-linecap="round"
													stroke-linejoin="round"
												>
													<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
													<path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
												</svg>
											</button>
											<!-- Delete -->
											<button
												onclick={(event) => onDelete(event, s)}
												class="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background text-destructive transition-colors hover:bg-surface"
												title="Delete student"
											>
												<svg
													class="size-3.5"
													viewBox="0 0 24 24"
													fill="none"
													stroke="currentColor"
													stroke-width="2"
													stroke-linecap="round"
													stroke-linejoin="round"
												>
													<polyline points="3 6 5 6 21 6" />
													<path
														d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"
													/>
												</svg>
											</button>
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>

		<div class="fixed right-6 bottom-6 z-10">
			<Pagination {currentPage} {totalPages} onPageChange={handlePageChange} />
		</div>
	</div>
</AppShell>

<StudentAttendanceModal
	open={attendanceModalOpen}
	student={viewingStudent}
	onClose={() => (attendanceModalOpen = false)}
/>

<!-- ── Add / Edit dialog ──────────────────────────────────────────────────── -->
{#if dialogOpen}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={closeDialog}
		onkeydown={(e) => e.key === 'Escape' && closeDialog()}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="dialog-title"
	>
		<div class="w-full max-w-2xl rounded-2xl border border-border bg-background shadow-xl">
			<div class="border-b border-border px-6 pt-6 pb-5">
				<h2 id="dialog-title" class="text-lg font-semibold">
					{editing ? 'Edit student' : 'Add student'}
				</h2>
				<p class="mt-1 text-sm text-muted-foreground">
					{editing
						? 'Update the student profile and class assignment.'
						: 'Add one student manually or paste a class list in one pass.'}
				</p>
			</div>

			<form onsubmit={onSubmit} class="space-y-5 p-6">
				{#if !editing}
					<div class="grid rounded-lg border border-border bg-surface p-1 sm:grid-cols-2">
						<button
							type="button"
							onclick={() => (entryMode = 'single')}
							class="rounded-md px-4 py-2 text-sm font-medium transition-colors {entryMode ===
							'single'
								? 'bg-background text-foreground shadow-sm'
								: 'text-muted-foreground hover:text-foreground'}"
						>
							Individual
						</button>
						<button
							type="button"
							onclick={() => (entryMode = 'bulk')}
							class="rounded-md px-4 py-2 text-sm font-medium transition-colors {entryMode ===
							'bulk'
								? 'bg-background text-foreground shadow-sm'
								: 'text-muted-foreground hover:text-foreground'}"
						>
							Bulk paste
						</button>
					</div>
				{/if}

				<div class="space-y-1.5">
					<label for="field-class" class="label-mono">Class / Section</label>
					<select
						id="field-class"
						bind:value={formClassId}
						required={classes.length > 0}
						class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
					>
						{#if classes.length === 0}
							<option value="">No classes available</option>
						{:else}
							<option value="" disabled>Select a class</option>
							{#each classes as c (c.id)}
								<option value={c.id}>{c.name}</option>
							{/each}
						{/if}
					</select>
					{#if classes.length === 0}
						<p class="mt-1 text-xs text-muted-foreground">
							Create a class first to assign students, or add student without class assignment.
						</p>
					{/if}
				</div>

				{#if !editing && entryMode === 'bulk'}
					<div class="space-y-2">
						<div class="flex items-center justify-between gap-3">
							<label for="bulk-students" class="label-mono">Student names</label>
							<span class="font-mono text-xs text-muted-foreground">
								{bulkNames.length}
								{bulkNames.length === 1 ? 'student' : 'students'}
							</span>
						</div>
						<textarea
							id="bulk-students"
							bind:value={bulkStudentNames}
							rows="10"
							placeholder="John Doe&#10;Jane Smith&#10;Michael Cruz"
							class="min-h-64 w-full resize-y rounded-md border border-border bg-background px-3 py-3 text-sm leading-6 focus:ring-2 focus:ring-primary focus:outline-none"
						></textarea>
						<p class="text-xs text-muted-foreground">
							Each new line creates one student. Student numbers and cards can be added later.
						</p>
					</div>
				{:else}
					<div class="grid gap-4 sm:grid-cols-2">
						<div class="space-y-1.5 sm:col-span-2">
							<label for="field-name" class="label-mono">Full name</label>
							<input
								id="field-name"
								bind:value={formName}
								required
								placeholder="Student full name"
								class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
							/>
						</div>
						<div class="space-y-1.5">
							<label for="field-number" class="label-mono">Student number (optional)</label>
							<input
								id="field-number"
								bind:value={formStudentNumber}
								placeholder="Add later"
								class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
							/>
						</div>
						<div class="space-y-1.5">
							<label for="field-card" class="label-mono">Card serial (optional)</label>
							<input
								id="field-card"
								bind:value={formCardSerial}
								placeholder="Pair later"
								class="w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm focus:ring-2 focus:ring-primary focus:outline-none"
							/>
						</div>
					</div>
				{/if}

				<div
					class="flex flex-col-reverse gap-2 border-t border-border pt-5 sm:flex-row sm:justify-end"
				>
					<button
						type="button"
						onclick={closeDialog}
						class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
					>
						Cancel
					</button>
					<button
						type="submit"
						disabled={!editing && entryMode === 'bulk' && bulkNames.length === 0}
						class="rounded-pill bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
					>
						{#if editing}
							Save Changes
						{:else if entryMode === 'bulk'}
							Add {bulkNames.length || ''} Students
						{:else}
							Add Student
						{/if}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

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
			class="w-full max-w-md space-y-5 rounded-2xl border border-border bg-background p-6 shadow-xl"
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
						bind:value={cardSerial}
						placeholder="Tap card on reader or type serial…"
						class="w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm focus:ring-2 focus:ring-primary focus:outline-none"
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

<!-- ── Delete confirmation dialog ────────────────────────────────────────── -->
{#if deleteTarget}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (deleteTarget = null)}
		onkeydown={(e) => e.key === 'Escape' && (deleteTarget = null)}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="delete-dialog-title"
	>
		<div
			class="w-full max-w-sm space-y-5 rounded-2xl border border-border bg-background p-6 shadow-xl"
		>
			<div class="flex flex-col items-center gap-3 text-center">
				<div class="flex size-12 items-center justify-center rounded-full bg-destructive/10">
					<svg
						class="size-6 text-destructive"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="3 6 5 6 21 6" />
						<path
							d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"
						/>
					</svg>
				</div>
				<div class="w-full text-left">
					<h2 id="delete-dialog-title" class="text-lg font-semibold">Delete student?</h2>
					<p class="mt-1 text-sm text-muted-foreground">
						<span class="font-medium text-foreground">{deleteTarget.name}</span> will be permanently removed.
					</p>
					<p class="mt-4 text-xs leading-relaxed text-muted-foreground">
						<strong class="font-semibold text-accent">PROTIP:</strong>
						<span class="block">
							You can hold down <strong class="font-semibold">Shift</strong> when clicking the delete
							button to bypass this confirmation entirely.
						</span>
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (deleteTarget = null)}
					class="flex-1 rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={() => confirmDelete()}
					class="flex-1 rounded-pill bg-destructive px-4 py-2 text-sm font-medium text-white hover:opacity-90"
				>
					Delete
				</button>
			</div>
		</div>
	</div>
{/if}

{#if toastMessage}
	<div
		class="fixed top-12 right-6 z-60 rounded-xl border border-border bg-background px-4 py-3 text-sm font-medium shadow-lg"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}

{#snippet emptyState()}
	<div class="mt-8 rounded-2xl border border-dashed border-border bg-surface/50 p-12 text-center">
		<p class="text-muted-foreground">
			{#if selectedClassId}
				No students assigned to this class yet.
			{:else}
				No students yet. Add your first student to begin.
			{/if}
		</p>
	</div>
{/snippet}

{#snippet td(value: string, extraClass?: string)}
	<td class="px-4 py-3 {extraClass ?? ''}">{value}</td>
{/snippet}
