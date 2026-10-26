<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { ArrowLeft, CalendarDays, CheckCircle2, UserX } from 'lucide-svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import DatePickerDialog from '$lib/components/ui/DatePickerDialog.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import {
		addEvent,
		deleteEvent,
		listClasses,
		listEvents,
		listStudents,
		type AttendanceEvent,
		type Class,
		type Student
	} from '$lib/api';
	import { fmtDate, fmtTime } from '$lib/csv';
	import { formatAttendanceDate, adjustDate } from '../attendance-state.svelte';

	let students = $state<Student[]>([]);
	let classes = $state<Class[]>([]);
	let events = $state<AttendanceEvent[]>([]);
	let selectedDate = $state(fmtDate(Date.now()));
	let loading = $state(true);
	let loadError = $state<string | null>(null);
	let savingStudentId = $state<string | null>(null);
	let absentTarget = $state<Student | null>(null);
	let absentReason = $state('');
	let toastMessage = $state<string | null>(null);
	let toastOk = $state(true);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;
	let datePickerOpen = $state(false);

	onMount(() => {
		reload();
	});

	const selectedDateLabel = $derived.by(() => {
		const formatted = formatAttendanceDate(selectedDate);
		const today = fmtDate(Date.now());
		if (selectedDate === today) return `Today \u2022 ${formatted}`;
		if (selectedDate === adjustDate(today, -1)) return `Yesterday \u2022 ${formatted}`;
		if (selectedDate === adjustDate(today, 1)) return `Tomorrow \u2022 ${formatted}`;
		return formatted;
	});

	const primaryClass = $derived(classes[0] ?? null);
	const roster = $derived(
		primaryClass ? students.filter((student) => student.classId === primaryClass.id) : students
	);
	const dayEvents = $derived.by(() =>
		events.filter((event) => {
			if (fmtDate(event.timestamp) !== selectedDate) return false;
			if (!primaryClass) return true;
			const student = students.find((item) => item.id === event.studentId);
			return event.classId === primaryClass.id || student?.classId === primaryClass.id;
		})
	);
	const presentStudentIds = $derived.by(
		() => new Set(dayEvents.filter((event) => event.type === 'in').map((event) => event.studentId))
	);
	const absentStudentIds = $derived.by(
		() =>
			new Set(dayEvents.filter((event) => event.type === 'absent').map((event) => event.studentId))
	);
	const presentCount = $derived(
		roster.filter((student) => presentStudentIds.has(student.id)).length
	);
	const absentStudents = $derived(roster.filter((student) => absentStudentIds.has(student.id)));
	const absentCount = $derived(absentStudents.length);
	const pendingCount = $derived(
		roster.filter(
			(student) => !presentStudentIds.has(student.id) && !absentStudentIds.has(student.id)
		).length
	);

	async function reload() {
		loading = true;
		loadError = null;
		try {
			const [loadedStudents, loadedClasses, loadedEvents] = await Promise.all([
				listStudents(),
				listClasses(),
				listEvents()
			]);
			students = loadedStudents;
			classes = loadedClasses;
			events = loadedEvents;
		} catch (error) {
			loadError = error instanceof Error ? error.message : 'Daily overview could not be loaded.';
		} finally {
			loading = false;
		}
	}

	function toast(message: string, ok = true) {
		toastMessage = message;
		toastOk = ok;
		if (toastTimer) clearTimeout(toastTimer);
		toastTimer = setTimeout(() => (toastMessage = null), 3000);
	}

	function studentEvents(student: Student) {
		return dayEvents
			.filter((event) => event.studentId === student.id)
			.sort((a, b) => eventTime(a) - eventTime(b));
	}

	function eventTime(event: AttendanceEvent) {
		return new Date(event.timestamp).getTime();
	}

	function classForStudent(student: Student) {
		return (
			classes.find((item) => item.id === (student.classId || primaryClass?.id)) ?? primaryClass
		);
	}

	function attendanceTimestamp(classItem: Class | null) {
		const time = classItem?.dayStart || '08:00';
		const date = new Date(`${selectedDate}T${time}:00`);
		return Number.isNaN(date.getTime()) ? new Date(`${selectedDate}T08:00:00`) : date;
	}

	async function markPresent(student: Student) {
		if (savingStudentId) return;
		if (presentStudentIds.has(student.id)) {
			toast(`${student.name} is already present`);
			return;
		}

		savingStudentId = student.id;
		const classItem = classForStudent(student);
		const timestamp = attendanceTimestamp(classItem);
		try {
			// Replacing an explicit absent mark keeps the student to one record.
			const existingAbsent = dayEvents.filter(
				(event) => event.studentId === student.id && event.type === 'absent'
			);
			if (existingAbsent.length > 0) {
				await Promise.all(
					existingAbsent.map((event) => deleteEvent(event.id, 'Daily overview mark present'))
				);
			}
			const created = await addEvent({
				studentId: student.id,
				classId: classItem?.id || student.classId,
				type: 'in',
				sessionKey: `${selectedDate}|${classItem?.id || student.classId || 'unassigned'}|day`,
				overrideReason: 'Daily overview manual present',
				timestamp: timestamp.toISOString()
			});
			events = [
				created,
				...events.filter((event) => !existingAbsent.some((item) => item.id === event.id))
			];
			toast(`${student.name} marked present`);
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Could not mark present';
			toast(`Could not mark present: ${msg}`, false);
		} finally {
			savingStudentId = null;
		}
	}

	function requestAbsent(student: Student) {
		if (!presentStudentIds.has(student.id)) return;
		absentTarget = student;
		absentReason = '';
	}

	async function confirmAbsent() {
		if (!absentTarget || savingStudentId) return;
		const reason = absentReason.trim();
		if (reason.length < 3) {
			toast('Absent change reason is required', false);
			return;
		}

		const target = absentTarget;
		const targetEvents = studentEvents(target);
		savingStudentId = target.id;
		try {
			await Promise.all(targetEvents.map((event) => deleteEvent(event.id, reason)));
			const classItem = classForStudent(target);
			const timestamp = attendanceTimestamp(classItem);
			// Absence is stored as an explicit 'absent' record (like the attendance
			// page) so SF2 renders an X and the mark survives navigation.
			const created = await addEvent({
				studentId: target.id,
				classId: classItem?.id || target.classId,
				type: 'absent',
				note: reason,
				sessionKey: `${selectedDate}|${classItem?.id || target.classId || 'unassigned'}|day`,
				timestamp: timestamp.toISOString()
			});
			events = [
				created,
				...events.filter((event) => !targetEvents.some((item) => item.id === event.id))
			];
			toast(`${target.name} marked absent`);
			absentTarget = null;
			absentReason = '';
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Could not mark absent';
			toast(`Could not mark absent: ${msg}`, false);
		} finally {
			savingStudentId = null;
		}
	}
</script>

<svelte:head>
	<title>Daily Overview - Attendance System</title>
	<meta name="description" content="Review and correct attendance by day." />
</svelte:head>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
	<PageHeader
		category="Attendance"
		title="Daily Attendance Overview"
		description={primaryClass
			? `Review ${primaryClass.name} attendance for the selected day.`
			: 'Review attendance for the selected day.'}
	>
		{#snippet actions()}
			<a href={resolve('/attendance/logs')} class="btn btn-secondary">
				<ArrowLeft class="size-4" aria-hidden="true" strokeWidth={1.9} />
				Back to logs
			</a>
			<button
				type="button"
				onclick={() => (datePickerOpen = true)}
				aria-haspopup="dialog"
				aria-expanded={datePickerOpen}
				class="btn btn-secondary"
				title={selectedDate}
			>
				<CalendarDays class="size-4" aria-hidden="true" strokeWidth={1.9} />
				<span>{selectedDateLabel}</span>
			</button>
		{/snippet}
	</PageHeader>

	<div class="min-h-0 flex-1 overflow-auto">
		<div class="page-frame flex flex-col gap-5">
			{#if loading}
				<LoadingBlock rows={3} label="Loading daily overview" />
			{:else if loadError}
				<EmptyState tone="warning" title="Daily overview unavailable" description={loadError}>
					{#snippet actions()}
						<button type="button" onclick={reload} class="btn btn-secondary control-ring">
							Retry
						</button>
					{/snippet}
				</EmptyState>
			{:else}
				<section class="grid gap-4 sm:grid-cols-4" aria-label="Daily attendance totals">
					{@render totalCard('Roster', roster.length, 'Students')}
					{@render totalCard('Present', presentCount, 'Present records', true)}
					{@render totalCard('Absent', absentCount, 'Marked absent')}
					{@render totalCard('Pending', pendingCount, 'Not yet recorded')}
				</section>

				<section class="surface-panel overflow-hidden">
					<div class="panel-header">
						<div>
							<h2 class="text-[15px] font-semibold tracking-[-0.012em]">{selectedDateLabel}</h2>
							<p class="mt-0.5 text-[12px] text-muted-foreground">
								Mark a learner present or absent for this day without class-hour checks.
							</p>
						</div>
						<span class="chip">{absentCount} absent</span>
					</div>

					<div class="panel-body">
						{#if roster.length === 0}
							<EmptyState
								title="No students in the class roster"
								description="Add students to the class list before reviewing daily attendance."
							/>
						{:else}
							<div class="grid gap-2 lg:grid-cols-2">
								{#each roster as student (student.id)}
									{@const present = presentStudentIds.has(student.id)}
									{@const absent = !present && absentStudentIds.has(student.id)}
									{@const records = studentEvents(student)}
									<div class="list-row flex min-w-0 items-center justify-between gap-3 p-3">
										<div class="min-w-0">
											<div class="text-balance-safe text-sm font-semibold">{student.name}</div>
											<div
												class="mt-1 font-mono text-[11px] {present
													? 'text-green-700'
													: absent
														? 'text-red-700'
														: 'text-muted-foreground'}"
											>
												{present && records[0]
													? `Present ${fmtTime(records[0].timestamp)}`
													: absent
														? 'Marked absent'
														: 'Not recorded'}
											</div>
										</div>
										<button
											type="button"
											disabled={savingStudentId !== null}
											onclick={() => (present ? requestAbsent(student) : markPresent(student))}
											class="btn btn-sm min-w-28 disabled:cursor-not-allowed disabled:opacity-60 {present
												? 'btn-secondary'
												: 'btn-primary'}"
										>
											{#if present}
												<UserX class="size-3.5" aria-hidden="true" />
												Mark absent
											{:else}
												<CheckCircle2 class="size-3.5" aria-hidden="true" />
												Mark present
											{/if}
										</button>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				</section>
			{/if}
		</div>
	</div>
</div>

<DatePickerDialog
	open={datePickerOpen}
	value={selectedDate}
	onClose={() => (datePickerOpen = false)}
	onSelect={({ date }) => {
		selectedDate = date || fmtDate(Date.now());
	}}
/>

<Dialog
	open={!!absentTarget}
	title="Mark Learner Absent"
	description="Removing the present record requires an audit reason."
	onClose={() => {
		if (!savingStudentId) absentTarget = null;
	}}
>
	{#if absentTarget}
		<div
			class="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface/60 px-3 py-2"
		>
			<span class="text-[13px] font-medium">{absentTarget.name}</span>
			<span class="font-mono text-[11px] text-muted-foreground tabular-nums">{selectedDate}</span>
		</div>
		<div class="space-y-1.5">
			<label for="absent-reason" class="text-[11.5px] font-medium text-muted-foreground"
				>Reason</label
			>
			<textarea
				id="absent-reason"
				rows="4"
				bind:value={absentReason}
				placeholder="Required for the audit log"
				class="textarea-field min-h-28 resize-none"
			></textarea>
		</div>
		<div class="flex justify-end gap-2 border-t border-border pt-3">
			<button
				type="button"
				disabled={savingStudentId !== null}
				onclick={() => (absentTarget = null)}
				class="btn btn-secondary"
			>
				Cancel
			</button>
			<button
				type="button"
				disabled={savingStudentId !== null || absentReason.trim().length < 3}
				onclick={confirmAbsent}
				class="btn btn-primary"
			>
				Save absent
			</button>
		</div>
	{/if}
</Dialog>

<FeedbackToast message={toastMessage} ok={toastOk} onClose={() => (toastMessage = null)} />

{#snippet totalCard(label: string, value: number, detail: string, accent = false)}
	<div class="metric-card {accent ? 'metric-card-accent' : ''}">
		<p class="text-[11.5px] font-medium text-muted-foreground">{label}</p>
		<div class="metric-value mt-2">{value}</div>
		<p class="mt-1.5 text-[11.5px] text-muted-foreground">{detail}</p>
	</div>
{/snippet}
