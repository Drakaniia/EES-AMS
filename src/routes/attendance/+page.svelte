<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import {
		listStudents,
		listClasses,
		findStudentByCard,
		lastEventForStudent,
		addEvent,
		deleteEvent,
		type Student,
		type Class
	} from '$lib/db-rust';
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
		eventId?: string;
	} | null>(null);

	let cardInput = $state('');

	// Toast
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;

	// Cooldown lock to prevent rapid duplicate taps
	let isProcessing = $state(false);

	// Undo tracking
	let lastEventId = $state<string | null>(null);
	let undoTimer: ReturnType<typeof setTimeout> | null = null;

	// ── Lifecycle ────────────────────────────────────────────────────────────
	onMount(() => {
		(async () => {
			await reload();

			const active = getActiveClass();
			if (active) {
				selectedClassId = active.id;
			}

			if (page.url.searchParams.get('manual') === 'true') {
				pickerOpen = true;
			}
		})();
	});

	async function reload() {
		const [s, c] = await Promise.all([listStudents(), listClasses()]);
		students = s;
		classes = c;
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

	let todayClasses = $derived.by(() => {
		const currentDay = new Date().getDay();
		return classes
			.filter((cls) => cls.days && cls.days.includes(currentDay))
			.sort((a, b) => {
				const [aH, aM] = a.dayStart.split(':').map(Number);
				const [bH, bM] = b.dayStart.split(':').map(Number);
				return aH * 60 + aM - (bH * 60 + bM);
			});
	});

	// ── Utility Functions ────────────────────────────────────────────────────────

	function getTimeOfDay(): 'Morning' | 'Afternoon' {
		const hour = new Date().getHours();
		return hour < 12 ? 'Morning' : 'Afternoon';
	}

	function getActiveClass(): Class | null {
		const now = new Date();
		const currentTime = now.getHours() * 60 + now.getMinutes();
		const currentDay = now.getDay();

		for (const cls of classes) {
			// Skip classes not scheduled for today
			if (cls.days && !cls.days.includes(currentDay)) continue;

			const [startHour, startMin] = cls.dayStart.split(':').map(Number);
			const [endHour, endMin] = cls.dayEnd.split(':').map(Number);
			const startTime = startHour * 60 + startMin;
			const endTime = endHour * 60 + endMin;

			if (currentTime >= startTime && currentTime <= endTime) {
				return cls;
			}
		}
		return null;
	}

	function endSession() {
		if (!selectedClassId) {
			import('$app/navigation').then((n) => n.goto('/'));
			return;
		}

		const classObj = classes.find((c) => c.id === selectedClassId);
		const classStudents = students.filter((s) => s.classId === selectedClassId);
		const total = classStudents.length;

		const presentCount = new Set(log.filter((l) => l.type === 'in').map((l) => l.studentNumber))
			.size;
		const summary = `${presentCount}/${total} students present`;

		import('$app/navigation').then((n) =>
			n.goto(
				`/?sessionEnd=true&summary=${encodeURIComponent(summary)}&className=${encodeURIComponent(classObj?.name || '')}`
			)
		);
	}

	// ── Dynamic Title Logic ────────────────────────────────────────────────────

	const activeClass = $derived(getActiveClass());
	const timeOfDay = $derived(getTimeOfDay());
	const dynamicTitle = $derived(() => {
		if (activeClass) {
			return `${timeOfDay} ${activeClass.name} Attendance`;
		}
		return 'Live Session';
	});

	const dynamicDescription = $derived(() => {
		if (activeClass) {
			return `Recording attendance for ${timeOfDay.toLowerCase()} ${activeClass.name} (${activeClass.dayStart} – ${activeClass.dayEnd})`;
		}
		return currentClass
			? `Recording attendance for ${currentClass.name} (${currentClass.dayStart} – ${currentClass.dayEnd})`
			: 'Active monitoring of student check-ins.';
	});

	// ── Helpers ──────────────────────────────────────────────────────────────
	function toast(msg: string, ok = true) {
		toastMessage = msg;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	async function undoLast() {
		if (!lastEventId || !lastResult) return;

		try {
			await deleteEvent(lastEventId);

			const eventIdToRemove = lastEventId;
			log = log.filter((l) => l.id !== eventIdToRemove);

			toast(`Undid ${lastResult.name} ${lastResult.type === 'in' ? 'check-in' : 'check-out'}`);
		} catch {
			toast('Failed to undo last action', false);
		} finally {
			lastEventId = null;
			lastResult = null;
			if (undoTimer) clearTimeout(undoTimer);
		}
	}

	function checkLate(classObj: Class | undefined, timestamp: number): boolean {
		if (!classObj) return false;

		const now = new Date(timestamp);
		const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

		let lateAfter = classObj.lateAfter;

		if (classObj.sessions && classObj.sessions.length > 0) {
			for (const session of classObj.sessions) {
				if (timeStr >= session.startTime && timeStr <= session.endTime) {
					lateAfter = session.lateAfter;
					break;
				}
			}
		}

		if (!lateAfter) return false;

		const [h, m] = lateAfter.split(':').map(Number);
		const lateTime = new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, m, 0, 0);

		return now > lateTime;
	}

	// ── Card input handler ───────────────────────────────────────────────────
	async function handleCardSubmit(serial: string) {
		const trimmed = serial.trim();
		if (!trimmed) return;

		// Prevent rapid duplicate taps
		if (isProcessing) {
			toast('Please wait — processing previous tap', false);
			return;
		}

		cardInput = '';
		isProcessing = true;

		try {
			const student = await findStudentByCard(trimmed);
			if (!student) {
				toast('Unknown card — not paired to any student', false);
				return;
			}
			await logForStudent(student);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			if (message.includes('duplicate check-in') || message.includes('already checked in')) {
				toast('Already checked in today — no duplicate allowed', false);
			} else {
				toast(`Error: ${message}`, false);
			}
		} finally {
			isProcessing = false;
		}
	}

	function isWithinClassHours(classObj: Class | undefined, timestamp: number): boolean {
		if (!classObj) return false;

		const now = new Date(timestamp);
		const timeStr = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

		if (classObj.sessions && classObj.sessions.length > 0) {
			for (const session of classObj.sessions) {
				if (timeStr >= session.startTime && timeStr <= session.endTime) {
					return true;
				}
			}
			return false;
		}

		return timeStr >= classObj.dayStart && timeStr <= classObj.dayEnd;
	}

	async function logForStudent(student: Student) {
		const studentClass = classes.find((c) => c.id === student.classId) || currentClass;

		if (!isWithinClassHours(studentClass, Date.now())) {
			toast('Not within class hours — attendance not allowed', false);
			return;
		}

		const last = await lastEventForStudent(student.id);
		const type: 'in' | 'out' = !last || last.type === 'out' ? 'in' : 'out';
		const ts = Date.now();

		const isLate = type === 'in' && checkLate(studentClass, ts);

		try {
			const createdEvent = await addEvent({
				studentId: student.id,
				classId: student.classId || selectedClassId || undefined,
				type,
				note: isLate ? 'Late' : undefined
			});
			lastEventId = createdEvent.id;

			lastResult = {
				ok: true,
				name: student.name,
				type,
				time: ts,
				isLate,
				eventId: createdEvent.id
			};
			log = [
				{
					id: createdEvent.id,
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
			if (undoTimer) clearTimeout(undoTimer);
			undoTimer = setTimeout(() => {
				lastResult = null;
				lastEventId = null;
			}, 5000);
		} catch (err: unknown) {
			const message = err instanceof Error ? err.message : String(err);
			if (message.includes('duplicate check-in') || message.includes('already checked in')) {
				toast(`${student.name} already checked in today`, false);
				return;
			}
			throw err;
		}
	}
</script>

<svelte:head>
	<title>Live Session — Attendance System</title>
</svelte:head>

<AppShell>
	<PageHeader category="Tap Mode" title={dynamicTitle()} description={dynamicDescription()}>
		{#snippet actions()}
			<div class="flex items-center gap-3">
				<button
					disabled={todayClasses.length === 0}
					onclick={() => {
						if (todayClasses.length === 0) {
							toast('No classes scheduled for today', false);
							return;
						}
						pickerQuery = '';
						pickerOpen = true;
					}}
					class="inline-flex items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
				>
					Manual log
				</button>

				<button
					onclick={endSession}
					class="inline-flex h-10 items-center gap-2 rounded-pill border border-border bg-background px-4 py-2 text-sm font-medium transition-colors hover:bg-surface"
				>
					End Session
				</button>
			</div>
		{/snippet}
	</PageHeader>

	<section
		class="grid min-h-[calc(100vh-28rem)] gap-8 px-6 py-10 md:px-12 lg:grid-cols-[1.2fr_1fr]"
	>
		<div
			class="relative flex min-h-[420px] items-center justify-center overflow-hidden rounded-3xl border border-border bg-surface p-10"
		>
			<div
				aria-hidden="true"
				class="pointer-events-none absolute inset-0 opacity-50"
				style="background: radial-gradient(60% 60% at 50% 40%, color-mix(in oklab, var(--primary) 22%, transparent), transparent 70%)"
			></div>

			{#if !selectedClassId}
				<div class="relative w-full max-w-md text-center">
					<h3 class="display-lg mb-2">No Classes Today</h3>
					<p class="mb-8 text-muted-foreground">
						No classes are scheduled for today. Configure class days in Settings.
					</p>
				</div>
			{:else}
				<div class="relative w-full max-w-md text-center">
					<div class="label-mono mb-4 text-primary">
						<span class="animate-pulse">●</span> Ready for card taps
					</div>

					<div
						class="mx-auto grid size-40 place-items-center rounded-full border-2 border-primary shadow-[0_0_30px_-5px_var(--primary)]"
					>
						<svg
							class="size-16 text-primary"
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

					<h3 class="display-lg mt-8">Tap a card</h3>
					<p class="mx-auto mt-2 max-w-md text-muted-foreground">
						Tap an ID card on the reader or type the serial below.
					</p>

					<form
						onsubmit={(e) => {
							e.preventDefault();
							handleCardSubmit(cardInput);
						}}
						class="mx-auto mt-6 max-w-sm"
					>
						<input
							type="text"
							bind:value={cardInput}
							placeholder="Tap card or enter serial…"
							class="w-full rounded-md border border-border bg-background px-4 py-3 text-center font-mono text-sm focus:ring-2 focus:ring-primary focus:outline-none"
						/>
					</form>
				</div>
			{/if}
		</div>

		<div class="flex h-full flex-col rounded-2xl border border-border bg-card p-6">
			<div class="mb-4 flex shrink-0 items-baseline justify-between">
				<div class="flex flex-col">
					<h3 class="text-lg font-medium">Session log</h3>
					<span class="label-mono text-xs opacity-60">Latest activity</span>
				</div>
				<form
					onsubmit={(e) => {
						e.preventDefault();
						handleCardSubmit(cardInput);
					}}
					class="w-56"
				>
					<input
						type="text"
						bind:value={cardInput}
						placeholder="Tap card or enter serial…"
						class="w-full rounded-pill border border-border bg-background px-4 py-1.5 font-mono text-xs focus:ring-2 focus:ring-primary focus:outline-none"
					/>
				</form>
			</div>

			<div class="flex-1 overflow-y-auto">
				{#if log.length === 0}
					<div
						class="flex h-full w-full flex-col items-center justify-center rounded-xl border border-dashed border-border p-4 text-center text-sm text-muted-foreground"
					>
						No activity recorded in this session.
					</div>
				{:else}
					<ul class="divide-y divide-border">
						{#each log as line (line.id)}
							<li class="flex items-center justify-between gap-3 py-3">
								<div class="min-w-0">
									<div class="truncate font-medium">{line.studentName}</div>
									<div class="label-mono">#{line.studentNumber} · {fmtTime(line.timestamp)}</div>
								</div>
								<div class="flex items-center gap-2">
									{#if line.isLate}
										<span
											class="rounded-pill border border-destructive/20 bg-destructive/10 px-2 py-0.5 font-mono text-[10px] font-bold text-destructive"
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
				? 'border-border bg-background text-foreground'
				: 'border-destructive bg-destructive text-destructive-foreground'}"
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
					<span class={lastResult.isLate ? 'font-bold text-destructive' : ''}>
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
		class="w-full rounded-md border border-border bg-background px-4 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
	/>

	<ul class="max-h-[300px] divide-y divide-border overflow-y-auto rounded-xl border border-border">
		{#if filteredStudents.length === 0}
			<li class="py-10 text-center text-sm text-muted-foreground">
				No students found {selectedClassId ? 'in this class' : ''}.
			</li>
		{:else}
			{#each filteredStudents as s (s.id)}
				<li>
					<button
						onclick={async () => {
							if (isProcessing) {
								toast('Please wait — processing previous request', false);
								return;
							}
							isProcessing = true;
							try {
								await logForStudent(s);
								pickerOpen = false;
							} catch {
								// Error already handled in logForStudent
							} finally {
								isProcessing = false;
							}
						}}
						class="flex w-full items-center justify-between px-4 py-3 text-left transition-colors hover:bg-surface"
					>
						<span>
							<div class="font-medium">{s.name}</div>
							<div class="label-mono text-xs opacity-60">#{s.studentNumber}</div>
						</span>
						<span class="label-mono text-xs font-bold text-primary">LOG →</span>
					</button>
				</li>
			{/each}
		{/if}
	</ul>

	<div class="flex justify-end pt-2">
		<button
			onclick={() => (pickerOpen = false)}
			class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
		>
			Close
		</button>
	</div>
</Dialog>

{#if toastMessage}
	<div
		class="fixed top-12 right-6 z-60 flex items-center gap-3 rounded-xl border px-4 py-3 text-sm font-medium shadow-lg
			{toastOk
			? 'border-border bg-background text-foreground'
			: 'border-destructive/40 bg-destructive/10 text-destructive'}"
		role="status"
		aria-live="polite"
	>
		<span>{toastMessage}</span>
		{#if lastEventId && toastOk}
			<button
				onclick={undoLast}
				class="rounded-md bg-primary/10 px-2 py-1 text-xs font-semibold text-primary transition-colors hover:bg-primary/20"
			>
				UNDO
			</button>
		{/if}
	</div>
{/if}

{#snippet pill(type: 'in' | 'out' | 'error')}
	<span
		class="shrink-0 rounded-pill px-2 py-1 font-mono text-[10px] font-bold
			{type === 'in'
			? 'bg-primary text-primary-foreground'
			: type === 'out'
				? 'border border-border bg-surface text-foreground'
				: 'bg-destructive text-destructive-foreground'}"
	>
		{type === 'in' ? 'IN' : type === 'out' ? 'OUT' : 'ERROR'}
	</span>
{/snippet}
