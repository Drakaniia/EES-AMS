<script lang="ts">
	import { onMount } from 'svelte';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import {
		listStudents,
		findStudentByCard,
		lastEventForStudent,
		addEvent,
		uid,
		type Student
	} from '$lib/db';
	import { NfcScanner, nfcSupported } from '$lib/nfc';
	import { fmtTime } from '$lib/csv';

	// ── Types ────────────────────────────────────────────────────────────────
	type LogLine = {
		id: string;
		studentName: string;
		studentNumber: string;
		type: 'in' | 'out' | 'error';
		message: string;
		timestamp: number;
	};

	// ── State ────────────────────────────────────────────────────────────────
	let scanning = $state(false);
	let log = $state<LogLine[]>([]);
	let students = $state<Student[]>([]);
	let pickerOpen = $state(false);
	let pickerQuery = $state('');
	let lastResult = $state<{ ok: boolean; name: string; type: 'in' | 'out'; time: number } | null>(
		null
	);

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	const supported = nfcSupported() === 'supported';

	// ── Derived ──────────────────────────────────────────────────────────────
	let filteredStudents = $derived(
		students.filter(
			(s) =>
				s.name.toLowerCase().includes(pickerQuery.toLowerCase()) ||
				s.studentNumber.includes(pickerQuery)
		)
	);

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
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
		await addEvent({ id: uid(), studentId: student.id, type, timestamp: ts });

		lastResult = { ok: true, name: student.name, type, time: ts };
		log = [
			{
				id: uid(),
				studentName: student.name,
				studentNumber: student.studentNumber,
				type,
				message: type === 'in' ? 'Checked in' : 'Checked out',
				timestamp: ts
			},
			...log
		].slice(0, 30);

		toast(`${student.name} · ${type === 'in' ? 'Checked in' : 'Checked out'}`);
		setTimeout(() => (lastResult = null), 2500);
	}

	function startScanning() {
		if (scanner) return;
		if (!supported) {
			toast('NFC not available on this device. Use manual entry.', false);
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

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		listStudents().then((s) => (students = s));
		return () => {
			scanner?.stop();
			scanner = null;
		};
	});
</script>

<svelte:head>
	<title>Tap Mode — Horizon Attendance</title>
	<meta
		name="description"
		content="Continuous NFC tap to check students in and out."
	/>
</svelte:head>

<AppShell>
	<PageHeader
		step="Step 03 · Live"
		title="Tap to attend"
		description="Hold an NFC student card to the back of the device. Each tap toggles between check-in and check-out."
	>
		{#snippet actions()}
			<button
				onclick={() => { pickerQuery = ''; pickerOpen = true; }}
				class="inline-flex items-center gap-2 px-4 py-2 rounded-pill border border-border bg-background text-sm font-medium hover:bg-surface transition-colors"
			>
				Manual log
			</button>

			{#if scanning}
				<button
					onclick={stopScanning}
					class="inline-flex items-center gap-2 px-4 py-2 rounded-pill border border-destructive/40 text-destructive text-sm font-medium hover:bg-destructive/10 transition-colors"
				>
					<!-- Square icon -->
					<svg class="size-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
						<rect x="3" y="3" width="18" height="18" rx="2" />
					</svg>
					Stop scanning
				</button>
			{:else}
				<button
					onclick={startScanning}
					class="inline-flex items-center gap-2 px-4 py-2 rounded-pill bg-primary text-primary-foreground text-sm font-medium hover:bg-accent transition-colors"
				>
					<!-- Play icon -->
					<svg class="size-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
						<polygon points="5,3 19,12 5,21" />
					</svg>
					Start scanning
				</button>
			{/if}
		{/snippet}
	</PageHeader>

	<section class="px-6 md:px-12 py-10 grid lg:grid-cols-[1.2fr_1fr] gap-8">
		<!-- ── Scanner panel ─────────────────────────────────────────────── -->
		<div
			class="relative rounded-3xl border border-border overflow-hidden bg-surface min-h-[420px] flex items-center justify-center p-10
				{scanning ? 'ring-2 ring-primary/40' : ''}"
		>
			<!-- Ambient glow -->
			<div
				aria-hidden="true"
				class="absolute inset-0 opacity-50 pointer-events-none"
				style="background: radial-gradient(60% 60% at 50% 40%, color-mix(in oklab, var(--primary) 22%, transparent), transparent 70%)"
			></div>

			<div class="relative text-center">
				<div class="label-mono mb-4">{scanning ? 'Listening for taps' : 'Scanner idle'}</div>

				<!-- Scan ring -->
				<div
					class="mx-auto size-40 rounded-full grid place-items-center border-2
						{scanning ? 'border-primary animate-pulse' : 'border-border'}"
				>
					<!-- ScanLine icon -->
					<svg
						class="size-16 {scanning ? 'text-primary' : 'text-muted-foreground'}"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="1.5"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<path d="M3 7V5a2 2 0 0 1 2-2h2" />
						<path d="M17 3h2a2 2 0 0 1 2 2v2" />
						<path d="M21 17v2a2 2 0 0 1-2 2h-2" />
						<path d="M7 21H5a2 2 0 0 1-2-2v-2" />
						<line x1="7" y1="12" x2="17" y2="12" />
					</svg>
				</div>

				<h3 class="display-lg mt-8">{scanning ? 'Tap a card' : 'Press start'}</h3>
				<p class="text-muted-foreground mt-2 max-w-md mx-auto">
					{#if supported}
						Cards are read via Web NFC. Keep the device awake while in tap mode.
					{:else}
						Web NFC isn't available — use Manual Log to record attendance.
					{/if}
				</p>
			</div>
		</div>

		<!-- ── Session log ───────────────────────────────────────────────── -->
		<div class="rounded-2xl border border-border bg-card p-6">
			<div class="flex items-baseline justify-between mb-4">
				<h3 class="text-lg font-medium">Session log</h3>
				<span class="label-mono">Latest taps</span>
			</div>

			{#if log.length === 0}
				{@render emptyLog()}
			{:else}
				<ul class="divide-y divide-border max-h-[420px] overflow-y-auto">
					{#each log as line (line.id)}
						<li class="py-3 flex items-center justify-between gap-3">
							<div class="min-w-0">
								<div class="font-medium truncate">{line.studentName}</div>
								<div class="label-mono">#{line.studentNumber} · {fmtTime(line.timestamp)}</div>
							</div>
							{@render pill(line.type)}
						</li>
					{/each}
				</ul>
			{/if}
		</div>
	</section>
</AppShell>

<!-- ── Last-result overlay ────────────────────────────────────────────────── -->
{#if lastResult}
	<div class="fixed inset-x-0 bottom-6 flex justify-center pointer-events-none px-4 z-50">
		<div
			class="pointer-events-auto rounded-2xl shadow-lg px-6 py-4 flex items-center gap-3 border
				{lastResult.ok
				? 'bg-primary text-primary-foreground border-primary'
				: 'bg-destructive text-destructive-foreground border-destructive'}"
		>
			{#if lastResult.ok}
				<!-- CheckCircle2 -->
				<svg class="size-6 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
					<polyline points="22 4 12 14.01 9 11.01" />
				</svg>
			{:else}
				<!-- XCircle -->
				<svg class="size-6 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
					<circle cx="12" cy="12" r="10" />
					<line x1="15" y1="9" x2="9" y2="15" />
					<line x1="9" y1="9" x2="15" y2="15" />
				</svg>
			{/if}
			<div>
				<div class="font-medium">{lastResult.name}</div>
				<div class="text-xs font-mono opacity-90">
					{lastResult.type === 'in' ? 'CHECK-IN' : 'CHECK-OUT'} · {fmtTime(lastResult.time)}
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- ── Manual log dialog ──────────────────────────────────────────────────── -->
{#if pickerOpen}
	<!-- Backdrop -->
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={() => (pickerOpen = false)}
		onkeydown={(e) => e.key === 'Escape' && (pickerOpen = false)}
	></div>

	<!-- Panel -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="picker-title"
	>
		<div class="w-full max-w-md rounded-2xl border border-border bg-background shadow-xl p-6 space-y-4">
			<div>
				<h2 id="picker-title" class="text-lg font-semibold">Manual log</h2>
				<p class="text-sm text-muted-foreground mt-1">Select a student to toggle their attendance.</p>
			</div>

			<!-- svelte-ignore a11y_autofocus -->
			<input
				autofocus
				placeholder="Search by name or number"
				bind:value={pickerQuery}
				class="w-full px-4 py-2 rounded-md border border-border bg-background text-sm focus:outline-none focus:ring-2 focus:ring-primary"
			/>

			<ul class="max-h-[300px] overflow-y-auto divide-y divide-border rounded-xl border border-border">
				{#if filteredStudents.length === 0}
					<li class="py-6 text-center text-sm text-muted-foreground">No matches</li>
				{:else}
					{#each filteredStudents as s (s.id)}
						<li>
							<button
								onclick={async () => {
									await logForStudent(s);
									pickerOpen = false;
								}}
								class="w-full text-left py-3 px-4 hover:bg-surface transition-colors flex items-center justify-between"
							>
								<span>
									<div class="font-medium">{s.name}</div>
									<div class="label-mono">#{s.studentNumber}</div>
								</span>
								<span class="label-mono text-primary">LOG →</span>
							</button>
						</li>
					{/each}
				{/if}
			</ul>

			<div class="flex justify-end">
				<button
					onclick={() => (pickerOpen = false)}
					class="px-4 py-2 rounded-md border border-border text-sm hover:bg-surface transition-colors"
				>
					Cancel
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- ── Toast ──────────────────────────────────────────────────────────────── -->
{#if toastMessage}
	<div
		class="fixed bottom-6 right-6 z-[60] px-4 py-3 rounded-xl border shadow-lg text-sm font-medium
			{toastOk
			? 'bg-background border-border text-foreground'
			: 'bg-destructive/10 border-destructive/40 text-destructive'}"
		role="status"
		aria-live="polite"
	>
		{toastMessage}
	</div>
{/if}

<!-- ── Snippets ───────────────────────────────────────────────────────────── -->
{#snippet emptyLog()}
	<div
		class="py-12 text-center text-muted-foreground text-sm border border-dashed border-border rounded-xl"
	>
		Nothing yet. Tap a card or use Manual Log.
	</div>
{/snippet}

{#snippet pill(type: 'in' | 'out' | 'error')}
	<span
		class="text-xs font-mono px-2 py-1 rounded-pill shrink-0
			{type === 'in'
			? 'bg-primary text-primary-foreground'
			: type === 'out'
			? 'bg-surface text-foreground border border-border'
			: 'bg-destructive text-destructive-foreground'}"
	>
		{type === 'in' ? 'CHECK-IN' : type === 'out' ? 'CHECK-OUT' : 'ERROR'}
	</span>
{/snippet}
