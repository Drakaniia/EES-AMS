<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import {
		listStudents,
		saveStudent,
		deleteStudent,
		uid,
		findStudentByCard,
		type Student
	} from '$lib/db';
	import { NfcScanner, nfcSupported } from '$lib/nfc';

	// ── State ────────────────────────────────────────────────────────────────
	let students = $state<Student[]>([]);
	let dialogOpen = $state(false);
	let editing = $state<Student | null>(null);
	let scanFor = $state<Student | null>(null);

	// Add/edit form fields
	let formName = $state('');
	let formStudentNumber = $state('');
	let formCardSerial = $state('');

	// Delete confirmation dialog
	let deleteTarget = $state<Student | null>(null);

	// Register-card dialog state
	let cardSerial = $state('');
	let scanning = $state(false);
	let cardError = $state<string | null>(null);

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	const supported = nfcSupported() === 'supported';

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string) {
		toastMessage = msg;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	async function reload() {
		students = await listStudents();
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		reload();
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
		if (!supported) return;

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
		dialogOpen = true;
	}

	function openEdit(s: Student) {
		editing = s;
		formName = s.name;
		formStudentNumber = s.studentNumber;
		formCardSerial = s.cardSerial ?? '';
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
		if (!name || !num) return;

		const base: Student = editing ?? {
			id: uid(),
			createdAt: Date.now(),
			name: '',
			studentNumber: ''
		};
		await saveStudent({ ...base, name, studentNumber: num, cardSerial: serial || undefined });
		toast(editing ? 'Student updated' : 'Student added');
		closeDialog();
		reload();
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
</script>

<svelte:head>
	<title>Students — Horizon Attendance</title>
	<meta name="description" content="Manage students and register their NFC cards." />
</svelte:head>

<AppShell>
	<PageHeader
		step="Step 02 · Roster"
		title="Distribute intelligent attendance cards"
		description="Add each student and pair an NFC ID card. Cards are identified by their unique serial — no app needed on the card itself."
	>
		{#snippet actions()}
			<button
				onclick={openAdd}
				class="inline-flex items-center gap-2 px-4 py-2 rounded-pill bg-primary text-primary-foreground text-sm font-medium hover:bg-accent transition-colors"
			>
				<!-- Plus icon -->
				<svg
					class="size-4"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<path d="M12 5v14M5 12h14" />
				</svg>
				Add student
			</button>
		{/snippet}
	</PageHeader>

	<!-- NFC status badge -->
	<div class="px-6 md:px-12 py-6">
		<div
			class="inline-flex items-center gap-2 text-xs font-mono px-3 py-2 rounded-pill border w-fit
				{supported
				? 'border-border bg-surface'
				: 'border-destructive/40 bg-destructive/10 text-destructive'}"
		>
			{#if supported}
				<!-- Wifi icon -->
				<svg
					class="size-3.5"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<path d="M5 12.55a11 11 0 0 1 14.08 0M1.42 9a16 16 0 0 1 21.16 0M8.53 16.11a6 6 0 0 1 6.95 0M12 20h.01" />
				</svg>
				NFC AVAILABLE ON THIS DEVICE
			{:else}
				<!-- WifiOff icon -->
				<svg
					class="size-3.5"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<line x1="1" y1="1" x2="23" y2="23" />
					<path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55M5 12.55a10.94 10.94 0 0 1 5.17-2.39M10.71 5.05A16 16 0 0 1 22.56 9M1.42 9a15.91 15.91 0 0 1 4.7-2.88M8.53 16.11a6 6 0 0 1 6.95 0M12 20h.01" />
				</svg>
				NFC UNAVAILABLE — USE MANUAL ENTRY
			{/if}
		</div>
	</div>

	<!-- Student roster -->
	<section class="px-6 md:px-12 pb-16">
		{#if students.length === 0}
			{@render emptyState()}
		{:else}
			<div class="rounded-2xl border border-border overflow-hidden bg-card">
				<table class="w-full text-sm">
					<thead class="bg-surface text-left">
						<tr>
							{@render th('Name')}
							{@render th('Student #')}
							{@render th('Card')}
							{@render th('Actions', 'w-36 text-right')}
						</tr>
					</thead>
					<tbody class="divide-y divide-border">
						{#each students as s (s.id)}
							<tr>
								{@render td(s.name, 'font-medium')}
								{@render td(s.studentNumber, 'font-mono')}
								<td class="px-4 py-3 font-mono text-xs">
									{#if s.cardSerial}
										<span class="px-2 py-1 rounded-pill bg-surface border border-border"
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
											class="inline-flex items-center justify-center size-8 rounded-md border border-border bg-background hover:bg-surface transition-colors"
											title="Pair NFC card"
											aria-label="Pair NFC card for {s.name}"
										>
											<svg
												class="size-3.5"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
											>
												<rect x="2" y="5" width="20" height="14" rx="2" />
												<path d="M2 10h20" />
											</svg>
										</button>
										<!-- Edit -->
										<button
											onclick={() => openEdit(s)}
											class="inline-flex items-center justify-center size-8 rounded-md border border-border bg-background hover:bg-surface transition-colors"
											title="Edit student"
											aria-label="Edit {s.name}"
										>
											<svg
												class="size-3.5"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
											>
												<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
												<path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
											</svg>
										</button>
										<!-- Delete -->
										<button
											onclick={() => onDelete(s)}
											class="inline-flex items-center justify-center size-8 rounded-md border border-border bg-background hover:bg-surface text-destructive transition-colors"
											title="Delete student"
											aria-label="Delete {s.name}"
										>
											<svg
												class="size-3.5"
												viewBox="0 0 24 24"
												fill="none"
												stroke="currentColor"
												stroke-width="2"
												stroke-linecap="round"
												stroke-linejoin="round"
												aria-hidden="true"
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
</AppShell>

<!-- ── Add / Edit dialog ──────────────────────────────────────────────────── -->
{#if dialogOpen}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={closeDialog}
		onkeydown={(e) => e.key === 'Escape' && closeDialog()}
	></div>

	<!-- Panel -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="dialog-title"
	>
		<div class="w-full max-w-md rounded-2xl border border-border bg-background shadow-xl p-6 space-y-5">
			<div>
				<h2 id="dialog-title" class="text-lg font-semibold">
					{editing ? 'Edit student' : 'Add student'}
				</h2>
				<p class="text-sm text-muted-foreground mt-1">
					Pair an NFC card now or later from the roster.
				</p>
			</div>

			<form onsubmit={onSubmit} class="space-y-4">
				<div class="space-y-1.5">
					<label for="field-name" class="label-mono">Full name</label>
					<input
						id="field-name"
						bind:value={formName}
						required
						class="w-full px-3 py-2 rounded-md border border-border bg-background text-sm focus:outline-none focus:ring-2 focus:ring-primary"
					/>
				</div>
				<div class="space-y-1.5">
					<label for="field-number" class="label-mono">Student number</label>
					<input
						id="field-number"
						bind:value={formStudentNumber}
						required
						class="w-full px-3 py-2 rounded-md border border-border bg-background text-sm focus:outline-none focus:ring-2 focus:ring-primary"
					/>
				</div>
				<div class="space-y-1.5">
					<label for="field-card" class="label-mono">Card serial (optional)</label>
					<input
						id="field-card"
						bind:value={formCardSerial}
						placeholder="e.g. 04:a3:b1:..."
						class="w-full px-3 py-2 rounded-md border border-border bg-background text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary"
					/>
				</div>
				<div class="flex justify-end gap-2 pt-1">
					<button
						type="button"
						onclick={closeDialog}
						class="px-4 py-2 rounded-md border border-border text-sm hover:bg-surface transition-colors"
					>
						Cancel
					</button>
					<button
						type="submit"
						class="px-4 py-2 rounded-pill bg-primary text-primary-foreground text-sm font-medium hover:bg-accent transition-colors"
					>
						{editing ? 'Save' : 'Add student'}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<!-- ── Register card dialog ───────────────────────────────────────────────── -->
{#if scanFor}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (scanFor = null)}
		onkeydown={(e) => e.key === 'Escape' && (scanFor = null)}
	></div>

	<!-- Panel -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="card-dialog-title"
	>
		<div class="w-full max-w-md rounded-2xl border border-border bg-background shadow-xl p-6 space-y-5">
			<div>
				<h2 id="card-dialog-title" class="text-lg font-semibold">Pair NFC card</h2>
				<p class="text-sm text-muted-foreground mt-1">Tap the card for {scanFor.name}.</p>
			</div>

			<div class="space-y-4">
				{#if !supported}
					<p class="text-xs text-destructive font-mono">
						Web NFC unavailable. Enter serial manually.
					</p>
				{/if}

				<!-- Scan area -->
				<div
					class="rounded-2xl border border-dashed border-border p-8 text-center bg-surface/50"
				>
					<svg
						class="size-10 mx-auto mb-3 {scanning
							? 'text-primary animate-pulse'
							: 'text-muted-foreground'}"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
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
					<div class="font-mono text-sm mt-2 break-all">{cardSerial || '—'}</div>
				</div>

				<!-- Manual entry -->
				<div class="space-y-1.5">
					<label for="manual-serial" class="label-mono">Or enter serial manually</label>
					<input
						id="manual-serial"
						bind:value={cardSerial}
						class="w-full px-3 py-2 rounded-md border border-border bg-background text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary"
					/>
				</div>

				{#if cardError}
					<p class="text-sm text-destructive">{cardError}</p>
				{/if}
			</div>

			<div class="flex justify-end gap-2">
				<button
					onclick={() => (scanFor = null)}
					class="px-4 py-2 rounded-md border border-border text-sm hover:bg-surface transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={onSaveCard}
					disabled={!cardSerial}
					class="px-4 py-2 rounded-pill bg-primary text-primary-foreground text-sm font-medium hover:bg-accent transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
				>
					Save
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- ── Delete confirmation dialog ────────────────────────────────────────── -->
{#if deleteTarget}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (deleteTarget = null)}
		onkeydown={(e) => e.key === 'Escape' && (deleteTarget = null)}
	></div>

	<!-- Panel -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="delete-dialog-title"
	>
		<div class="w-full max-w-sm rounded-2xl border border-border bg-background shadow-xl p-6 space-y-5">
			<!-- Icon + heading -->
			<div class="flex flex-col items-center text-center gap-3">
				<div class="size-12 rounded-full bg-destructive/10 flex items-center justify-center">
					<svg
						class="size-6 text-destructive"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<polyline points="3 6 5 6 21 6" />
						<path
							d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6M9 6V4a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2"
						/>
					</svg>
				</div>
				<div>
					<h2 id="delete-dialog-title" class="text-lg font-semibold">Delete student?</h2>
					<p class="text-sm text-muted-foreground mt-1">
						<span class="font-medium text-foreground">{deleteTarget.name}</span> and all their
						attendance records will be permanently removed. This cannot be undone.
					</p>
				</div>
			</div>

			<div class="flex gap-2">
				<button
					onclick={() => (deleteTarget = null)}
					class="flex-1 px-4 py-2 rounded-md border border-border text-sm hover:bg-surface transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={confirmDelete}
					class="flex-1 px-4 py-2 rounded-pill bg-destructive text-white text-sm font-medium hover:opacity-90 transition-opacity"
				>
					Delete
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- ── Toast ──────────────────────────────────────────────────────────────── -->
{#if toastMessage}
	<div
		class="fixed bottom-6 right-6 z-[60] px-4 py-3 rounded-xl border border-border bg-background shadow-lg text-sm font-medium"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}

<!-- ── Snippets ───────────────────────────────────────────────────────────── -->
{#snippet emptyState()}
	<div class="rounded-2xl border border-dashed border-border bg-surface/50 p-12 text-center">
		<p class="text-muted-foreground">No students yet. Add your first student to begin.</p>
	</div>
{/snippet}

{#snippet th(label: string, extraClass?: string)}
	<th class="px-4 py-3 label-mono {extraClass ?? ''}">{label}</th>
{/snippet}

{#snippet td(value: string, extraClass?: string)}
	<td class="px-4 py-3 {extraClass ?? ''}">{value}</td>
{/snippet}
