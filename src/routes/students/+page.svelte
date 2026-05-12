<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import {
		listStudents,
		saveStudent,
		deleteStudent,
		listClasses,
		findStudentByCard,
		type Student,
		type Class
	} from '$lib/db-rust';
	import { NfcScanner, nfcSupported } from '$lib/nfc';
	import { resolve } from '$app/paths';

	// ── State ────────────────────────────────────────────────────────────────
	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let selectedClassId = $state<string>(''); // Filter
	let searchTerms = $state('');
	let sortBy = $state<'name' | 'number' | 'date'>('name');
	let sortOrder = $state<'asc' | 'desc'>('asc');

	let dialogOpen = $state(false);
	let editing = $state<Student | null>(null);
	let scanFor = $state<Student | null>(null);

	// Add/edit form fields
	let formName = $state('');
	let formStudentNumber = $state('');
	let formCardSerial = $state('');
	let formClassId = $state('');

	// Delete confirmation dialog
	let deleteTarget = $state<Student | null>(null);

	// Register-card dialog state
	let cardSerial = $state('');
	let scanning = $state(false);
	let cardError = $state<string | null>(null);

	// Toast
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
	const filteredStudents = $derived(() => {
		let result = students;

		// Search
		if (searchTerms.trim()) {
			const term = searchTerms.toLowerCase();
			result = result.filter(
				(s) =>
					s.name.toLowerCase().includes(term) ||
					s.studentNumber.toLowerCase().includes(term) ||
					s.cardSerial?.toLowerCase().includes(term)
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
	const totalPages = $derived(Math.ceil(filteredStudents().length / itemsPerPage));
	const paginatedStudents = $derived(() => {
		const start = (currentPage - 1) * itemsPerPage;
		const end = start + itemsPerPage;
		return filteredStudents().slice(start, end);
	});

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
			s.studentNumber,
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
		link.setAttribute('download', 'students_roster.csv');
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

	// ── NFC scanner for register-card dialog ─────────────────────────────────
	let scanner: NfcScanner | null = null;

	$effect(() => {
		if (!scanFor) {
			cardSerial = '';
			cardError = null;
			scanning = false;
			scanner?.stop();
			scanner = null;
			return;
		}

		(async () => {
			try {
				const supported = await nfcSupported();
				if (!supported) {
					cardError = 'NFC Card Reader not connected. Connect USB reader or enter serial manually.';
					scanning = false;
					return;
				}

				scanning = true;
				const student = scanFor;
				scanner = new NfcScanner(
					async (s) => {
						cardSerial = s;
						scanning = false;
						const existing = await findStudentByCard(s);
						if (existing && existing.id !== student.id) {
							cardError = `This card is already paired to ${existing.name}.`;
						}
						scanner?.stop();
					},
					(e) => {
						cardError = e.message;
						scanning = false;
					}
				);
				scanner.start();
			} catch {
				cardError = 'Failed to check NFC Card Reader. Please try again.';
				scanning = false;
			}
		})();

		return () => {
			scanner?.stop();
			scanner = null;
		};
	});

	// ── Dialog helpers ───────────────────────────────────────────────────────
	function openAdd() {
		editing = null;
		formName = '';
		formStudentNumber = '';
		formCardSerial = '';
		formClassId = selectedClassId || (classes.length > 0 ? classes[0].id : '');
		dialogOpen = true;
	}

	function openEdit(s: Student) {
		editing = s;
		formName = s.name;
		formStudentNumber = s.studentNumber;
		formCardSerial = s.cardSerial ?? '';
		formClassId = s.classId ?? '';
		dialogOpen = true;
	}

	function closeDialog() {
		dialogOpen = false;
		editing = null;
	}

	async function onSubmit(e: SubmitEvent) {
		e.preventDefault();
		const name = formName.trim();
		const num = formStudentNumber.trim();
		const serial = formCardSerial.trim().toLowerCase();
		const classId = formClassId;

		console.log('Form submission:', { name, num, serial, classId, editing });

		if (!name || !num) {
			console.log('Validation failed: name or number missing');
			toast('Please fill in all required fields');
			return;
		}

		// Check for duplicate student number (only for new students)
		if (!editing) {
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
						studentNumber: num,
						cardSerial: serial || undefined,
						classId: classId || undefined
					}
				: {
						// For new students, pass empty string as ID to trigger creation
						id: '',
						createdAt: new Date().toISOString(),
						name,
						studentNumber: num,
						cardSerial: serial || undefined,
						classId: classId || undefined
					};

			console.log('Saving student:', studentData);

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

	async function onDelete(s: Student) {
		deleteTarget = s;
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		await deleteStudent(deleteTarget.id);
		toast('Deleted');
		deleteTarget = null;
		reload();
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
	<meta name="description" content="Manage students and register their NFC cards." />
</svelte:head>

<AppShell>
	<div class="flex h-full flex-col overflow-hidden">
		<PageHeader
			category="Students"
			title="Student Roster"
			description="Manage your student list and their NFC identification cards."
		>
			{#snippet actions()}
				<div class="flex items-center gap-3">
					<a
						href={resolve('/records')}
						class="border-border hover:bg-surface inline-flex h-10 items-center gap-2 rounded-md border px-4 py-2 text-sm font-medium transition-colors"
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
						class="border-border hover:bg-surface inline-flex h-10 items-center gap-2 rounded-md border px-4 py-2 text-sm font-medium transition-colors"
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
						class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex h-10 items-center gap-2 px-6 py-2 text-sm font-medium transition-colors"
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
						class="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2"
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
						placeholder="Name, number, or card serial..."
						class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border pr-4 pl-10 text-sm focus:ring-2 focus:outline-none"
					/>
				</div>
			</div>

			<!-- Class Filter -->
			<div class="space-y-2">
				<div class="label-mono">Filter by Class</div>
				<select
					bind:value={selectedClassId}
					class="border-border bg-background focus:ring-primary h-10 w-full rounded-md border px-3 text-sm focus:ring-2 focus:outline-none"
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
					{filteredStudents().length}
					<span class="text-muted-foreground ml-2 text-xs font-normal">
						(out of {students.length})
					</span>
				</div>
			</div>
		</section>

		<!-- Student roster -->
		<section class="min-h-0 flex-1 px-6 pb-20 md:px-12" bind:clientHeight={availableHeight}>
			{#if students.length === 0}
				{@render emptyState()}
			{:else}
				<div class="border-border bg-card mt-8 overflow-hidden rounded-2xl border">
					<table class="w-full text-sm">
						<thead class="bg-surface text-left">
							<tr>
								<th class="label-mono px-4 py-3">
									<button
										onclick={() => toggleSort('name')}
										class="hover:text-primary inline-flex items-center gap-1 transition-colors"
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
										class="hover:text-primary inline-flex items-center gap-1 transition-colors"
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
						<tbody class="divide-border divide-y">
							{#each paginatedStudents() as s (s.id)}
								<tr>
									{@render td(s.name, 'font-medium')}
									{@render td(s.studentNumber, 'font-mono')}
									<td class="px-4 py-3">
										<span class="rounded-pill bg-surface border-border border px-2 py-0.5 text-xs">
											{getClassName(s.classId)}
										</span>
									</td>
									<td class="px-4 py-3 font-mono text-xs">
										{#if s.cardSerial}
											<span class="rounded-pill bg-surface border-border border px-2 py-1"
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
												class="border-border bg-background hover:bg-surface inline-flex size-8 items-center justify-center rounded-md border transition-colors"
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
												class="border-border bg-background hover:bg-surface inline-flex size-8 items-center justify-center rounded-md border transition-colors"
												title="Pair NFC card"
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
												class="border-border bg-background hover:bg-surface inline-flex size-8 items-center justify-center rounded-md border transition-colors"
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
												onclick={() => onDelete(s)}
												class="border-border bg-background hover:bg-surface text-destructive inline-flex size-8 items-center justify-center rounded-md border transition-colors"
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
		<div
			class="border-border bg-background w-full max-w-md space-y-5 rounded-2xl border p-6 shadow-xl"
		>
			<div>
				<h2 id="dialog-title" class="text-lg font-semibold">
					{editing ? 'Edit student' : 'Add student'}
				</h2>
				<p class="text-muted-foreground mt-1 text-sm">
					Assign to a class and pair an NFC card later.
				</p>
			</div>

			<form onsubmit={onSubmit} class="space-y-4">
				<div class="space-y-1.5">
					<label for="field-name" class="label-mono">Full name</label>
					<input
						id="field-name"
						bind:value={formName}
						required
						class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
					/>
				</div>
				<div class="space-y-1.5">
					<label for="field-number" class="label-mono">Student number</label>
					<input
						id="field-number"
						bind:value={formStudentNumber}
						required
						class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
					/>
				</div>
				<div class="space-y-1.5">
					<label for="field-class" class="label-mono">Class / Section</label>
					<select
						id="field-class"
						bind:value={formClassId}
						required={classes.length > 0}
						class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 text-sm focus:ring-2 focus:outline-none"
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
						<p class="text-muted-foreground mt-1 text-xs">
							Create a class first to assign students, or add student without class assignment.
						</p>
					{/if}
				</div>
				<div class="space-y-1.5">
					<label for="field-card" class="label-mono">Card serial (optional)</label>
					<input
						id="field-card"
						bind:value={formCardSerial}
						placeholder="e.g. 04:a3:b1:..."
						class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 font-mono text-sm focus:ring-2 focus:outline-none"
					/>
				</div>
				<div class="flex justify-end gap-2 pt-1">
					<button
						type="button"
						onclick={closeDialog}
						class="border-border hover:bg-surface rounded-md border px-4 py-2 text-sm transition-colors"
					>
						Cancel
					</button>
					<button
						type="submit"
						class="rounded-pill bg-primary text-primary-foreground hover:bg-accent px-4 py-2 text-sm font-medium transition-colors"
					>
						{editing ? 'Save Changes' : 'Add Student'}
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
			class="border-border bg-background w-full max-w-md space-y-5 rounded-2xl border p-6 shadow-xl"
		>
			<div>
				<h2 id="card-dialog-title" class="text-lg font-semibold">Pair NFC card</h2>
				<p class="text-muted-foreground mt-1 text-sm">Tap the card for {scanFor.name}.</p>
			</div>

			<div class="space-y-4">
				<div class="border-border bg-surface/50 rounded-2xl border border-dashed p-8 text-center">
					<svg
						class="mx-auto mb-3 size-10 {scanning
							? 'text-primary animate-pulse'
							: 'text-muted-foreground'}"
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
					<div class="label-mono">
						{#if scanning}
							Waiting for tap…
						{:else if cardSerial}
							Card detected
						{:else}
							Idle
						{/if}
					</div>
					<div class="mt-2 font-mono text-sm break-all">{cardSerial || '—'}</div>
				</div>

				<div class="space-y-1.5">
					<label for="manual-serial" class="label-mono">Or enter serial manually</label>
					<input
						id="manual-serial"
						bind:value={cardSerial}
						class="border-border bg-background focus:ring-primary w-full rounded-md border px-3 py-2 font-mono text-sm focus:ring-2 focus:outline-none"
					/>
				</div>

				{#if cardError}
					<p class="text-destructive text-sm">{cardError}</p>
				{/if}
			</div>

			<div class="flex justify-end gap-2">
				<button
					onclick={() => (scanFor = null)}
					class="border-border hover:bg-surface rounded-md border px-4 py-2 text-sm transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={onSaveCard}
					disabled={!cardSerial}
					class="rounded-pill bg-primary text-primary-foreground hover:bg-accent px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50"
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
			class="border-border bg-background w-full max-w-sm space-y-5 rounded-2xl border p-6 shadow-xl"
		>
			<div class="flex flex-col items-center gap-3 text-center">
				<div class="bg-destructive/10 flex size-12 items-center justify-center rounded-full">
					<svg
						class="text-destructive size-6"
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
				<div>
					<h2 id="delete-dialog-title" class="text-lg font-semibold">Delete student?</h2>
					<p class="text-muted-foreground mt-1 text-sm">
						<span class="text-foreground font-medium">{deleteTarget.name}</span> will be permanently removed.
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (deleteTarget = null)}
					class="border-border hover:bg-surface flex-1 rounded-md border px-4 py-2 text-sm transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={confirmDelete}
					class="rounded-pill bg-destructive flex-1 px-4 py-2 text-sm font-medium text-white hover:opacity-90"
				>
					Delete
				</button>
			</div>
		</div>
	</div>
{/if}

{#if toastMessage}
	<div
		class="border-border bg-background fixed right-6 bottom-6 z-60 rounded-xl border px-4 py-3 text-sm font-medium shadow-lg"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}

{#snippet emptyState()}
	<div class="border-border bg-surface/50 rounded-2xl border border-dashed p-12 text-center">
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
