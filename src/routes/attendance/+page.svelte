<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import {
		listStudents,
		listClasses,
		findStudentByCard,
		lastEventForStudent,
		addEvent,
		uid,
		type Student,
		type Class
	} from '$lib/db-rust';
	import { NfcScanner, nfcSupported } from '$lib/nfc';
	import { fmtTime } from '$lib/csv';

	// ── Types ────────────────────────────────────────────────────────────────
	type LogLine = {
		id: string;
		studentName: string;
		studentNumber: string;
		type: 'in' | 'out' | 'error';
		isLate?: boolean;
		message: string;
		timestamp: number;
	};

	// ── State ────────────────────────────────────────────────────────────────
	let scanning = $state(false);
	let log = $state<LogLine[]>([]);
	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let selectedClassId = $state<string>('');

	let pickerOpen = $state(false);
	let pickerQuery = $state('');
	let lastResult = $state<{
		ok: boolean;
		name: string;
		type: 'in' | 'out';
		time: number;
		isLate?: boolean;
	} | null>(null);

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// NFC support state
	let supported = $state<'connected' | 'disconnected'>('disconnected');
	let supportedLoading = $state(true);

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		(async () => {
			try {
				supported = await nfcSupported();
			} catch {
				supported = 'disconnected';
			} finally {
				supportedLoading = false;
			}
			await reload();
		})();

		return () => {
			scanner?.stop();
			scanner = null;
		};
	});

	async function reload() {
		const [s, c] = await Promise.all([listStudents(), listClasses()]);
		students = s;
		classes = c;
		// Auto-select first class if none selected
		if (!selectedClassId && c.length > 0) {
			selectedClassId = c[0].id;
		}
	}

	// ── Derived ──────────────────────────────────────────────────────────────
	let filteredStudents = $derived(
		students.filter((s) => {
			const matchesQuery =
				s.name.toLowerCase().includes(pickerQuery.toLowerCase()) ||
				s.studentNumber.includes(pickerQuery);
			const matchesClass = !selectedClassId || s.classId === selectedClassId;
			return matchesQuery && matchesClass;
		})
	);

	let currentClass = $derived(classes.find((c) => c.id === selectedClassId));

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	function checkLate(classObj: Class | undefined, timestamp: number): boolean {
		if (!classObj || !classObj.lateAfter) return false;

		const now = new Date(timestamp);
		const [h, m] = classObj.lateAfter.split(':').map(Number);
		const originalDate = new Date(timestamp);
		const lateTime = new Date(
			originalDate.getFullYear(),
			originalDate.getMonth(),
			originalDate.getDate(),
			h,
			m,
			0,
			0
		);

		return now > lateTime;
	}

	// ── NFC scanner ──────────────────────────────────────────────────────────
	let scanner: NfcScanner | null = null;

	async function handleSerial(serial: string) {
		const student = await findStudentByCard(serial);
		if (!student) {
			const line: LogLine = {
				id: uid(),
				studentName: 'Unknown card',
				studentNumber: serial,
				type: 'error',
				message: 'Not paired to any student',
				timestamp: Date.now()
			};
			log = [line, ...log].slice(0, 30);
			toast('Unknown card — not paired to any student', false);
			return;
		}
		await logForStudent(student);
	}

	async function logForStudent(student: Student) {
		const last = await lastEventForStudent(student.id);
		const type: 'in' | 'out' = !last || last.type === 'out' ? 'in' : 'out';
		const ts = Date.now();

		// Determine if late (only for check-in)
		const studentClass = classes.find((c) => c.id === student.classId) || currentClass;
		const isLate = type === 'in' && checkLate(studentClass, ts);

		await addEvent({
			studentId: student.id,
			classId: student.classId || selectedClassId || undefined,
			type,
			note: isLate ? 'Late' : undefined
		});

		lastResult = { ok: true, name: student.name, type, time: ts, isLate };
		log = [
			{
				id: uid(),
				studentName: student.name,
				studentNumber: student.studentNumber,
				type,
				isLate,
				message: type === 'in' ? (isLate ? 'Checked in (LATE)' : 'Checked in') : 'Checked out',
				timestamp: ts
			},
			...log
		].slice(0, 30);

		toast(
			`${student.name} · ${type === 'in' ? (isLate ? 'LATE' : 'Checked in') : 'Checked out'}`,
			!isLate
		);
		setTimeout(() => (lastResult = null), 2500);
	}

	function startScanning() {
		if (scanner) return;
		if (supported === 'disconnected') {
			toast('NFC Card Reader not connected.', false);
			return;
		}
		scanner = new NfcScanner(handleSerial, (e) => toast(e.message, false));
		scanner.start();
		scanning = true;
	}

	function stopScanning() {
		scanner?.stop();
		scanner = null;
		scanning = false;
	}
</script>

<svelte:head>
	<title>Live Session — Attendance System</title>
</svelte:head>

<AppShell>
	<PageHeader
		category="Tap Mode"
		title="Live Session"
		description={currentClass
			? `Recording attendance for ${currentClass.name} (${currentClass.dayStart} – ${currentClass.dayEnd})`
			: 'Active monitoring of student check-ins.'}
	>
		{#snippet actions()}
			<div class="flex items-center gap-3">
				<!-- Class Selector -->
				<select
					bind:value={selectedClassId}
					class="border-border bg-background focus:ring-primary rounded-pill h-10 border px-4 py-2 text-sm focus:ring-2 focus:outline-none"
				>
					<option value="">No Active Class</option>
					{#each classes as c (c.id)}
						<option value={c.id}>{c.name}</option>
					{/each}
				</select>

				<button
					onclick={() => {
						pickerQuery = '';
						pickerOpen = true;
					}}
					class="rounded-pill border-border bg-background hover:bg-surface inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
				>
					Manual log
				</button>

				{#if scanning}
					<button
						onclick={stopScanning}
						class="rounded-pill border-destructive/40 text-destructive hover:bg-destructive/10 inline-flex items-center gap-2 border px-4 py-2 text-sm font-medium transition-colors"
					>
						<svg class="size-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
							<rect x="3" y="3" width="18" height="18" rx="2" />
						</svg>
						Stop
					</button>
				{:else}
					<button
						onclick={startScanning}
						class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex items-center gap-2 px-4 py-2 text-sm font-medium transition-colors"
					>
						<svg class="size-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
							<polygon points="5,3 19,12 5,21" />
						</svg>
						Start
					</button>
				{/if}
			</div>
		{/snippet}
	</PageHeader>

	<section class="grid gap-8 px-6 py-10 md:px-12 lg:grid-cols-[1.2fr_1fr]">
		<div
			class="border-border bg-surface relative flex min-h-[420px] items-center justify-center overflow-hidden rounded-3xl border p-10
				{scanning ? 'ring-primary/40 ring-2' : ''}"
		>
			<div
				aria-hidden="true"
				class="pointer-events-none absolute inset-0 opacity-50"
				style="background: radial-gradient(60% 60% at 50% 40%, color-mix(in oklab, var(--primary) 22%, transparent), transparent 70%)"
			></div>

			<div class="relative text-center">
				<div class="label-mono mb-4">
					{#if scanning}
						<span class="text-primary animate-pulse">●</span> Listening for taps
					{:else}
						Scanner idle
					{/if}
				</div>

				<div
					class="mx-auto grid size-40 place-items-center rounded-full border-2
						{scanning ? 'border-primary animate-pulse shadow-[0_0_30px_-5px_var(--primary)]' : 'border-border'}"
				>
					<svg
						class="size-16 {scanning ? 'text-primary' : 'text-muted-foreground'}"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.5"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M3 7V5a2 2 0 0 1 2-2h2" />
						<path d="M17 3h2a2 2 0 0 1 2 2v2" />
						<path d="M21 17v2a2 2 0 0 1-2 2h-2" />
						<path d="M7 21H5a2 2 0 0 1-2-2v-2" />
						<line x1="7" y1="12" x2="17" y2="12" />
					</svg>
				</div>

				<h3 class="display-lg mt-8">{scanning ? 'Tap a card' : 'Press start'}</h3>
				<p class="text-muted-foreground mx-auto mt-2 max-w-md">
					{#if supportedLoading}
						Checking hardware…
					{:else if supported === 'connected'}
						USB NFC Card Reader detected. Keep the device awake.
					{:else}
						NFC Card Reader not found. Use manual log or check connection.
					{/if}
				</p>
			</div>
		</div>

		<div class="border-border bg-card flex flex-col rounded-2xl border p-6">
			<div class="mb-4 flex items-baseline justify-between">
				<h3 class="text-lg font-medium">Session log</h3>
				<span class="label-mono">Latest activity</span>
			</div>

			<div class="flex-1 overflow-y-auto">
				{#if log.length === 0}
					<div
						class="text-muted-foreground border-border rounded-xl border border-dashed py-12 text-center text-sm"
					>
						No activity recorded in this session.
					</div>
				{:else}
					<ul class="divide-border divide-y">
						{#each log as line (line.id)}
							<li class="flex items-center justify-between gap-3 py-3">
								<div class="min-w-0">
									<div class="truncate font-medium">{line.studentName}</div>
									<div class="label-mono">#{line.studentNumber} · {fmtTime(line.timestamp)}</div>
								</div>
								<div class="flex items-center gap-2">
									{#if line.isLate}
										<span
											class="rounded-pill bg-destructive/10 text-destructive border-destructive/20 border px-2 py-0.5 font-mono text-[10px] font-bold"
										>
											LATE
										</span>
									{/if}
									{@render pill(line.type)}
								</div>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
	</section>
</AppShell>

<!-- ── Overlays ───────────────────────────────────────────────────────────── -->
{#if lastResult}
	<div class="pointer-events-none fixed inset-x-0 bottom-10 z-50 flex justify-center px-4">
		<div
			class="pointer-events-auto flex items-center gap-4 rounded-3xl border px-8 py-5 shadow-2xl
				{lastResult.ok
				? 'bg-background border-border text-foreground'
				: 'bg-destructive text-destructive-foreground border-destructive'}"
		>
			<div
				class="grid size-12 place-items-center rounded-full
				{lastResult.type === 'in'
					? lastResult.isLate
						? 'bg-destructive/20 text-destructive'
						: 'bg-primary/20 text-primary'
					: 'bg-surface text-muted-foreground'}"
			>
				{#if lastResult.type === 'in'}
					<svg
						class="size-6"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.5"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="20 6 9 17 4 12" />
					</svg>
				{:else}
					<svg
						class="size-6"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2.5"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M18 6 6 18" />
						<path d="m6 6 12 12" />
					</svg>
				{/if}
			</div>
			<div>
				<div class="text-xl font-bold">{lastResult.name}</div>
				<div class="label-mono flex gap-2">
					<span class={lastResult.isLate ? 'text-destructive font-bold' : ''}>
						{lastResult.type === 'in' ? (lastResult.isLate ? 'LATE' : 'CHECK-IN') : 'CHECK-OUT'}
					</span>
					<span class="text-muted-foreground">·</span>
					<span class="text-muted-foreground">{fmtTime(lastResult.time)}</span>
				</div>
			</div>
		</div>
	</div>
{/if}

<Dialog
	open={pickerOpen}
	title="Manual log"
	description="Search for a student to manually record their attendance."
	on:close={() => (pickerOpen = false)}
>
	<input
		placeholder="Search name or student number…"
		bind:value={pickerQuery}
		class="border-border bg-background focus:ring-primary w-full rounded-md border px-4 py-2 text-sm focus:ring-2 focus:outline-none"
	/>

	<ul class="divide-border border-border max-h-[300px] divide-y overflow-y-auto rounded-xl border">
		{#if filteredStudents.length === 0}
			<li class="text-muted-foreground py-10 text-center text-sm">
				No students found {selectedClassId ? 'in this class' : ''}.
			</li>
		{:else}
			{#each filteredStudents as s (s.id)}
				<li>
					<button
						onclick={async () => {
							await logForStudent(s);
							pickerOpen = false;
						}}
						class="hover:bg-surface flex w-full items-center justify-between px-4 py-3 text-left transition-colors"
					>
						<span>
							<div class="font-medium">{s.name}</div>
							<div class="label-mono text-xs opacity-60">#{s.studentNumber}</div>
						</span>
						<span class="label-mono text-primary text-xs font-bold">LOG →</span>
					</button>
				</li>
			{/each}
		{/if}
	</ul>

	<div class="flex justify-end pt-2">
		<button
			onclick={() => (pickerOpen = false)}
			class="border-border hover:bg-surface rounded-md border px-4 py-2 text-sm transition-colors"
		>
			Close
		</button>
	</div>
</Dialog>

{#if toastMessage}
	<div
		class="fixed right-6 bottom-6 z-60 rounded-xl border px-4 py-3 text-sm font-medium shadow-lg
			{toastOk
			? 'bg-background border-border text-foreground'
			: 'bg-destructive/10 border-destructive/40 text-destructive'}"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}

{#snippet pill(type: 'in' | 'out' | 'error')}
	<span
		class="rounded-pill shrink-0 px-2 py-1 font-mono text-[10px] font-bold
			{type === 'in'
			? 'bg-primary text-primary-foreground'
			: type === 'out'
				? 'bg-surface text-foreground border-border border'
				: 'bg-destructive text-destructive-foreground'}"
	>
		{type === 'in' ? 'IN' : type === 'out' ? 'OUT' : 'ERROR'}
	</span>
{/snippet}
