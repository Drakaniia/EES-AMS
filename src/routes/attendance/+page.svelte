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
	<meta name="description" content="Continuous NFC tap to check students in and out." />
</svelte:head>

<AppShell>
	<PageHeader
		step="Step 03 · Live"
		title="Tap to attend"
		description="Hold an NFC student card to the back of the device. Each tap toggles between check-in and check-out."
	>
		{#snippet actions()}
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
					<!-- Square icon -->
					<svg class="size-4" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
						<rect x="3" y="3" width="18" height="18" rx="2" />
					</svg>
					Stop scanning
				</button>
			{:else}
				<button
					onclick={startScanning}
					class="rounded-pill bg-primary text-primary-foreground hover:bg-accent inline-flex items-center gap-2 px-4 py-2 text-sm font-medium transition-colors"
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

	<section class="grid gap-8 px-6 py-10 md:px-12 lg:grid-cols-[1.2fr_1fr]">
		<!-- ── Scanner panel ─────────────────────────────────────────────── -->
		<div
			class="border-border bg-surface relative flex min-h-[420px] items-center justify-center overflow-hidden rounded-3xl border p-10
				{scanning ? 'ring-primary/40 ring-2' : ''}"
		>
			<!-- Ambient glow -->
			<div
				aria-hidden="true"
				class="pointer-events-none absolute inset-0 opacity-50"
				style="background: radial-gradient(60% 60% at 50% 40%, color-mix(in oklab, var(--primary) 22%, transparent), transparent 70%)"
			></div>

			<div class="relative text-center">
				<div class="label-mono mb-4">{scanning ? 'Listening for taps' : 'Scanner idle'}</div>

				<!-- Scan ring -->
				<div
					class="mx-auto grid size-40 place-items-center rounded-full border-2
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
				<p class="text-muted-foreground mx-auto mt-2 max-w-md">
					{#if supported}
						Cards are read via Web NFC. Keep the device awake while in tap mode.
					{:else}
						Web NFC isn't available — use Manual Log to record attendance.
					{/if}
				</p>
			</div>
		</div>

		<!-- ── Session log ───────────────────────────────────────────────── -->
		<div class="border-border bg-card rounded-2xl border p-6">
			<div class="mb-4 flex items-baseline justify-between">
				<h3 class="text-lg font-medium">Session log</h3>
				<span class="label-mono">Latest taps</span>
			</div>

			{#if log.length === 0}
				{@render emptyLog()}
			{:else}
				<ul class="divide-border max-h-[420px] divide-y overflow-y-auto">
					{#each log as line (line.id)}
						<li class="flex items-center justify-between gap-3 py-3">
							<div class="min-w-0">
								<div class="truncate font-medium">{line.studentName}</div>
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
	<div class="pointer-events-none fixed inset-x-0 bottom-6 z-50 flex justify-center px-4">
		<div
			class="pointer-events-auto flex items-center gap-3 rounded-2xl border px-6 py-4 shadow-lg
				{lastResult.ok
				? 'bg-primary text-primary-foreground border-primary'
				: 'bg-destructive text-destructive-foreground border-destructive'}"
		>
			{#if lastResult.ok}
				<!-- CheckCircle2 -->
				<svg
					class="size-6 shrink-0"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
					<polyline points="22 4 12 14.01 9 11.01" />
				</svg>
			{:else}
				<!-- XCircle -->
				<svg
					class="size-6 shrink-0"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
					aria-hidden="true"
				>
					<circle cx="12" cy="12" r="10" />
					<line x1="15" y1="9" x2="9" y2="15" />
					<line x1="9" y1="9" x2="15" y2="15" />
				</svg>
			{/if}
			<div>
				<div class="font-medium">{lastResult.name}</div>
				<div class="font-mono text-xs opacity-90">
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
		<div
			class="border-border bg-background w-full max-w-md space-y-4 rounded-2xl border p-6 shadow-xl"
		>
			<div>
				<h2 id="picker-title" class="text-lg font-semibold">Manual log</h2>
				<p class="text-muted-foreground mt-1 text-sm">
					Select a student to toggle their attendance.
				</p>
			</div>

			<!-- svelte-ignore a11y_autofocus -->
			<input
				autofocus
				placeholder="Search by name or number"
				bind:value={pickerQuery}
				class="border-border bg-background focus:ring-primary w-full rounded-md border px-4 py-2 text-sm focus:ring-2 focus:outline-none"
			/>

			<ul
				class="divide-border border-border max-h-[300px] divide-y overflow-y-auto rounded-xl border"
			>
				{#if filteredStudents.length === 0}
					<li class="text-muted-foreground py-6 text-center text-sm">No matches</li>
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
					class="border-border hover:bg-surface rounded-md border px-4 py-2 text-sm transition-colors"
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
		class="fixed right-6 bottom-6 z-[60] rounded-xl border px-4 py-3 text-sm font-medium shadow-lg
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
		class="text-muted-foreground border-border rounded-xl border border-dashed py-12 text-center text-sm"
	>
		Nothing yet. Tap a card or use Manual Log.
	</div>
{/snippet}

{#snippet pill(type: 'in' | 'out' | 'error')}
	<span
		class="rounded-pill shrink-0 px-2 py-1 font-mono text-xs
			{type === 'in'
			? 'bg-primary text-primary-foreground'
			: type === 'out'
				? 'bg-surface text-foreground border-border border'
				: 'bg-destructive text-destructive-foreground'}"
	>
		{type === 'in' ? 'CHECK-IN' : type === 'out' ? 'CHECK-OUT' : 'ERROR'}
	</span>
{/snippet}
