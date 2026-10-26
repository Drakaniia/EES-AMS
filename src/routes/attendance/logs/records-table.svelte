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
		availableHeight = $bindable(0)
	}: {
		paginatedRecords: StudentAttendance[];
		groupedAttendance: StudentAttendance[];
		currentPage: number;
		totalPages: number;
		onEdit: (record: StudentAttendance) => void;
		onAudit: (record: StudentAttendance) => void;
		onDelete: (event: MouseEvent, record: StudentAttendance) => void;
		onPageChange: (page: number) => void;
		availableHeight?: number;
	} = $props();
</script>

<section
	class="flex min-h-0 flex-1 flex-col px-5 pb-20 md:px-8 lg:px-10"
	bind:clientHeight={availableHeight}
>
	<div class="table-wrap min-h-0 flex-1">
		<table class="min-w-[720px] text-[13px]">
			<thead class="text-left">
				<tr>
					<th class="w-28 px-4 py-2.5">Date</th>
					<th class="px-4 py-2.5">Student</th>
					<th class="w-32 px-4 py-2.5">Class</th>
					<th class="w-40 px-4 py-2.5">Check in</th>
					<th class="w-28 px-4 py-2.5 text-right">Actions</th>
				</tr>
			</thead>
			<tbody>
				{#if groupedAttendance.length === 0}
					<tr>
						<td colspan={5} class="px-4 py-14 text-center">
							<p class="text-[13px] font-medium">No attendance records match these filters</p>
							<p class="mt-1 text-[12px] text-muted-foreground">
								Widen the date range, or switch the class back to All.
							</p>
						</td>
					</tr>
				{:else}
					{#each paginatedRecords as record (record.studentId + record.date)}
						<tr>
							<td
								class="px-4 py-2.5 align-middle font-mono text-[12px] text-muted-foreground tabular-nums"
							>
								{record.date}
							</td>
							<td class="px-4 py-2.5 align-middle">
								<div class="text-balance-safe font-medium">{record.studentName}</div>
							</td>
							<td class="px-4 py-2.5 align-middle">
								<span class="chip">{record.className}</span>
							</td>
							<td class="px-4 py-2.5 align-middle">
								{#if record.checkInTime}
									<div class="flex flex-col items-start gap-1">
										{@render checkInPill(record.checkInTime, record.isLate)}
										{#if primaryEvent(record)?.overrideReason}
											<span class="max-w-56 text-[11px] leading-4 text-muted-foreground">
												{primaryEvent(record)?.overrideReason}
											</span>
										{/if}
									</div>
								{:else}
									<span class="text-[12px] text-muted-foreground">—</span>
								{/if}
							</td>
							<td class="px-4 py-2.5 align-middle">
								{#if record.events.length > 0}
									<div class="flex items-center justify-end gap-0.5">
										<button
											type="button"
											onclick={() => onEdit(record)}
											aria-label="Edit attendance record"
											class="icon-btn size-7"
											title="Edit record"
										>
											<Pencil class="size-3.5" strokeWidth={1.9} aria-hidden="true" />
										</button>
										<button
											type="button"
											onclick={() => onAudit(record)}
											aria-label="View audit history"
											class="icon-btn size-7"
											title="Audit history"
										>
											<History class="size-3.5" strokeWidth={1.9} aria-hidden="true" />
										</button>
										<button
											type="button"
											onclick={(event) => onDelete(event, record)}
											aria-label="Delete attendance record"
											class="icon-btn size-7 hover:bg-destructive/10 hover:text-destructive"
											title="Delete record"
										>
											<Trash2 class="size-3.5" strokeWidth={1.9} aria-hidden="true" />
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
	<Pagination {currentPage} {totalPages} {onPageChange} />
</div>

{#snippet checkInPill(time: string, isLate?: boolean)}
	<span
		class="inline-flex items-center rounded-md px-1.5 py-0.5 font-mono text-[11px] font-semibold tabular-nums
			{isLate
			? 'bg-destructive/10 text-destructive'
			: 'bg-emerald-500/10 text-emerald-800 dark:text-emerald-300'}"
	>
		{time}{#if isLate}<span class="ml-1 font-sans text-[10px]">Late</span>{/if}
	</span>
{/snippet}
