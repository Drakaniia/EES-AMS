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

	// ── State ────────────────────────────────────────────────────────────────
	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let selectedClassId = $state<string>(''); // Filter

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
	let itemsPerPage = $state(10);

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string) {
		toastMessage = msg;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	// Computed pagination values
	const totalPages = $derived(Math.ceil(students.length / itemsPerPage));
	const paginatedStudents = $derived(() => {
		const start = (currentPage - 1) * itemsPerPage;
		const end = start + itemsPerPage;
		return students.slice(start, end);
	});

	function handlePageChange(page: number) {
		currentPage = page;
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
	<PageHeader
		category="Students"
		title="Student Roster"
		description="Manage your student list and their NFC identification cards."
	>
		{#snippet actions()}
			<div class="flex items-center gap-3">
				<!-- Class Filter -->
				<select
					bind:value={selectedClassId}
					class="border-border bg-background focus:ring-primary rounded-pill h-10 border px-4 py-2 text-sm focus:ring-2 focus:outline-none"
				>
					<option value="">All Classes</option>
					{#each classes as c (c.id)}
						<option value={c.id}>{c.name}</option>
					{/each}
				</select>

				<button
					onclick={openAdd}
					class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex items-center gap-2 px-4 py-2 text-sm font-medium transition-colors"
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

	<!-- Student roster -->
	<section class="px-6 pb-16 md:px-12">
		{#if students.length === 0}
			{@render emptyState()}
		{:else}
			<div class="border-border bg-card mt-8 overflow-hidden rounded-2xl border">
				<table class="w-full text-sm">
					<thead class="bg-surface text-left">
						<tr>
							{@render th('Name')}
							{@render th('Student #')}
							{@render th('Class')}
							{@render th('Card')}
							{@render th('Actions', 'w-36 text-right')}
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

{#snippet th(label: string, extraClass?: string)}
	<th class="label-mono px-4 py-3 {extraClass ?? ''}">{label}</th>
{/snippet}

{#snippet td(value: string, extraClass?: string)}
	<td class="px-4 py-3 {extraClass ?? ''}">{value}</td>
{/snippet}
