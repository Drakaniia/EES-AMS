<script lang="ts">
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import { Pencil, History, Trash2 } from 'lucide-svelte';
	import type { StudentAttendance } from './records-state.svelte';
	import { primaryEvent } from './records-state.svelte';

	let {
		paginatedRecords,
		groupedAttendance,
		currentPage,
		totalPages,
		onEdit,
		onAudit,
		onDelete,
		onPageChange,
	}: {
		paginatedRecords: StudentAttendance[];
		groupedAttendance: StudentAttendance[];
		currentPage: number;
		totalPages: number;
		onEdit: (record: StudentAttendance) => void;
		onAudit: (record: StudentAttendance) => void;
		onDelete: (event: MouseEvent, record: StudentAttendance) => void;
		onPageChange: (page: number) => void;
	} = $props();
</script>

<section class="min-h-0 flex-1 px-4 pb-20 md:px-8 lg:px-10">
	<div class="table-wrap">
		<table class="min-w-[720px] text-sm">
			<thead class="bg-surface text-left">
				<tr>
					<th class="label-mono px-4 py-3">Date</th>
					<th class="label-mono px-4 py-3">Student</th>
					<th class="label-mono px-4 py-3">Class</th>
					<th class="label-mono px-4 py-3">Check In</th>
					<th class="label-mono w-36 px-4 py-3 text-right">Actions</th>
				</tr>
			</thead>
			<tbody class="divide-y divide-border">
				{#if groupedAttendance.length === 0}
					<tr>
						<td colspan={5} class="px-4 py-12 text-center text-muted-foreground">
							No attendance records match the filters.
						</td>
					</tr>
				{:else}
					{#each paginatedRecords as record (record.studentId + record.date)}
						<tr class="transition-colors hover:bg-surface/40">
							<td class="px-4 py-3 align-top font-mono">{record.date}</td>
							<td class="px-4 py-3 align-top">
								<div class="text-balance-safe font-medium">{record.studentName}</div>
							</td>
							<td class="px-4 py-3 align-top">
								<span
									class="rounded-pill border border-border bg-surface px-2 py-0.5 text-[10px]"
								>
									{record.className}
								</span>
							</td>
							<td class="px-4 py-3 align-top">
								{#if record.checkInTime}
									<div class="flex flex-col items-start gap-1">
										{@render checkInPill(record.checkInTime, record.isLate)}
										{#if primaryEvent(record)?.overrideReason}
											<span class="max-w-56 text-xs leading-5 text-muted-foreground">
												{primaryEvent(record)?.overrideReason}
											</span>
										{/if}
									</div>
								{:else}
									<span class="font-mono text-xs text-muted-foreground">—</span>
								{/if}
							</td>
							<td class="px-4 py-3 text-right align-top">
								{#if record.events.length > 0}
									<div class="inline-flex items-center gap-1">
										<button
											type="button"
											onclick={() => onEdit(record)}
											aria-label="Edit attendance record"
											class="inline-flex size-8 items-center justify-center rounded-md border border-border text-primary transition-colors hover:bg-primary/10"
										>
											<Pencil class="size-3.5" aria-hidden="true" />
										</button>
										<button
											type="button"
											onclick={() => onAudit(record)}
											aria-label="View audit history"
											class="inline-flex size-8 items-center justify-center rounded-md border border-border text-muted-foreground transition-colors hover:bg-surface"
										>
											<History class="size-3.5" aria-hidden="true" />
										</button>
										<button
											type="button"
											onclick={(event) => onDelete(event, record)}
											aria-label="Delete attendance record"
											class="inline-flex size-8 items-center justify-center rounded-md border border-border text-destructive transition-colors hover:bg-destructive/10"
										>
											<Trash2 class="size-3.5" aria-hidden="true" />
										</button>
									</div>
								{/if}
							</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</table>
	</div>
</section>

<div class="fixed bottom-6 left-1/2 z-30 -translate-x-1/2">
	<Pagination {currentPage} {totalPages} onPageChange={onPageChange} />
</div>

{#snippet checkInPill(time: string, isLate?: boolean)}
	<span
		class="rounded-pill px-2 py-1 font-mono text-xs
					{isLate ? 'bg-destructive text-destructive-foreground' : 'bg-primary text-primary-foreground'}"
	>
		{time}
		{#if isLate}
			(LATE){/if}
	</span>
{/snippet}
