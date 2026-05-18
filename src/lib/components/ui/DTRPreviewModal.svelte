<script lang="ts">
	import type { Student, Class, AttendanceEvent } from '$lib/db-rust';
	import { exportDtrExcel } from '$lib/db-rust';

	interface Props {
		open: boolean;
		onClose: () => void;
		student: Student;
		classData?: Class;
		events: AttendanceEvent[];
		month: number;
		year: number;
	}

	let { open, onClose, student, classData, events, month, year }: Props = $props();

	let exporting = $state(false);
	let error = $state<string | null>(null);

	const monthNames = [
		'January', 'February', 'March', 'April', 'May', 'June',
		'July', 'August', 'September', 'October', 'November', 'December'
	];

	async function handleExport() {
		exporting = true;
		error = null;
		try {
			await exportDtrExcel(student, classData, events, month, year);
			onClose();
		} catch (e: any) {
			error = e.toString();
		} finally {
			exporting = false;
		}
	}

	// Group events by day for preview
	let days = $derived.by(() => {
		const daysMap = new Map<number, { in?: string; out?: string }>();
		events.forEach((event: AttendanceEvent) => {
			const dt = new Date(event.timestamp);
			const day = dt.getDate();
			if (!daysMap.has(day)) daysMap.set(day, {});
			const d = daysMap.get(day)!;
			const time = dt.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });
			
			if (event.type === 'in') {
				if (!d.in || time < d.in) d.in = time;
			} else {
				if (!d.out || time > d.out) d.out = time;
			}
		});
		return daysMap;
	});

	// Get number of days in month
	let daysInMonth = $derived(new Date(year, month, 0).getDate());
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 backdrop-blur-sm"
		role="dialog"
		aria-modal="true"
	>
		<div class="flex max-h-[90vh] w-full max-w-4xl flex-col rounded-2xl border border-border bg-background shadow-2xl">
			<!-- Header -->
			<div class="flex items-center justify-between border-b border-border px-6 py-4">
				<h2 class="text-xl font-bold">DTR Export Preview</h2>
				<button
					onclick={onClose}
					class="rounded-full p-2 transition-colors hover:bg-surface"
					aria-label="Close"
				>
					<svg class="size-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<path d="M18 6L6 18M6 6l12 12" />
					</svg>
				</button>
			</div>

			<!-- Content -->
			<div class="flex-1 overflow-y-auto p-6">
				<div class="mb-6 grid gap-4 rounded-xl bg-surface p-4 sm:grid-cols-2">
					<div>
						<div class="text-xs font-bold uppercase tracking-wider text-muted-foreground">Student</div>
						<div class="text-lg font-medium">{student.name}</div>
						<div class="text-sm text-muted-foreground">{student.studentNumber}</div>
					</div>
					<div>
						<div class="text-xs font-bold uppercase tracking-wider text-muted-foreground">Period</div>
						<div class="text-lg font-medium">{monthNames[month - 1]} {year}</div>
						<div class="text-sm text-muted-foreground">{events.length} records found</div>
					</div>
				</div>

				<div class="overflow-hidden rounded-xl border border-border">
					<table class="w-full text-left text-sm">
						<thead class="bg-surface text-xs font-bold uppercase tracking-wider text-muted-foreground">
							<tr>
								<th class="px-4 py-3">Day</th>
								<th class="px-4 py-3">A.M. Arrival</th>
								<th class="px-4 py-3">A.M. Departure</th>
								<th class="px-4 py-3">P.M. Arrival</th>
								<th class="px-4 py-3">P.M. Departure</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-border">
							{#each Array.from({ length: daysInMonth }, (_, i) => i + 1) as day}
								{@const d = days.get(day)}
								<tr class="hover:bg-surface/50">
									<td class="px-4 py-2 font-medium">{day}</td>
									<td class="px-4 py-2">{d?.in && parseInt(d.in.split(':')[0]) < 12 ? d.in : '-'}</td>
									<td class="px-4 py-2">-</td>
									<td class="px-4 py-2">{d?.in && parseInt(d.in.split(':')[0]) >= 12 ? d.in : '-'}</td>
									<td class="px-4 py-2">{d?.out || '-'}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				{#if error}
					<div class="mt-4 rounded-lg bg-red-500/10 p-3 text-sm text-red-500">
						{error}
					</div>
				{/if}
			</div>

			<!-- Footer -->
			<div class="flex items-center justify-end gap-3 border-t border-border px-6 py-4">
				<button
					onclick={onClose}
					class="rounded-pill border border-border px-6 py-2 text-sm font-medium transition-colors hover:bg-surface"
				>
					Cancel
				</button>
				<button
					onclick={handleExport}
					disabled={exporting}
					class="inline-flex items-center gap-2 rounded-pill bg-primary px-8 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:opacity-50"
				>
					{#if exporting}
						<svg class="size-4 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M21 12a9 9 0 1 1-6.219-8.56" />
						</svg>
						Exporting...
					{:else}
						<svg class="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
							<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
							<polyline points="7 10 12 15 17 10" />
							<line x1="12" y1="15" x2="12" y2="3" />
						</svg>
						Save Excel
					{/if}
				</button>
			</div>
		</div>
	</div>
{/if}
