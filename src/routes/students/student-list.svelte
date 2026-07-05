<script lang="ts">
	import { resolve } from '$app/paths';
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import type { Student } from '$lib/db-rust';
	import { genderLabel } from './student-state.svelte';

	let {
		students,
		paginatedStudents,
		searchTerms,
		sortBy,
		sortOrder,
		currentPage,
		totalPages,
		maleStudentCount,
		femaleStudentCount,
		filteredStudents,
		assignedClassLabel,
		canCreateStudents,
		studentCreationBlockedMessage,
		onSearchChange,
		onToggleSort,
		onPageChange,
		onOpenAttendance,
		onOpenEdit,
		onOpenScan,
		onDelete,
	}: {
		students: Student[];
		paginatedStudents: Student[];
		searchTerms: string;
		sortBy: 'name' | 'date';
		sortOrder: 'asc' | 'desc';
		currentPage: number;
		totalPages: number;
		maleStudentCount: number;
		femaleStudentCount: number;
		filteredStudents: Student[];
		assignedClassLabel: string;
		canCreateStudents: boolean;
		studentCreationBlockedMessage: string;
		onSearchChange: (value: string) => void;
		onToggleSort: (field: 'name' | 'date') => void;
		onPageChange: (page: number) => void;
		onOpenAttendance: (student: Student) => void;
		onOpenEdit: (student: Student) => void;
		onOpenScan: (student: Student) => void;
		onDelete: (event: MouseEvent, student: Student) => void;
	} = $props();

	let availableHeight = $state(0);
</script>

<!-- Tools Bar -->
<section class="grid gap-4 px-4 pt-5 md:grid-cols-2 md:px-8 lg:grid-cols-3 lg:px-10">
	<div class="space-y-2">
		<div class="label-mono">Search Students</div>
		<div class="relative">
			<svg
				class="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<circle cx="11" cy="11" r="8" />
				<path d="m21 21-4.3-4.3" />
			</svg>
			<input
				type="text"
				value={searchTerms}
				oninput={(e) => onSearchChange((e.currentTarget as HTMLInputElement).value)}
				placeholder="Search by name..."
				class="h-10 w-full rounded-md border border-border bg-background pr-4 pl-10 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
			/>
		</div>
	</div>

	<div class="space-y-2">
		<div class="label-mono">Class / Section</div>
		<div
			class="flex h-10 items-center rounded-md border border-border bg-surface px-3 text-sm font-medium"
		>
			{assignedClassLabel}
		</div>
	</div>

	<div class="space-y-2">
		<div class="label-mono">Total Students</div>
		<div class="flex h-10 items-center justify-between gap-3">
			<div class="font-mono text-lg font-bold">
				{filteredStudents.length}
				<span class="ml-2 text-xs font-normal text-muted-foreground">
					(out of {students.length})
				</span>
			</div>
			<div class="flex shrink-0 items-center gap-2 font-mono text-xs">
				<span class="rounded-pill border border-border bg-surface px-2 py-1">
					M {maleStudentCount}
				</span>
				<span class="rounded-pill border border-border bg-surface px-2 py-1">
					F {femaleStudentCount}
				</span>
			</div>
		</div>
	</div>
</section>

<!-- Class List -->
<section class="min-h-0 flex-1 px-4 pb-20 md:px-8 lg:px-10" bind:clientHeight={availableHeight}>
	{#if students.length === 0}
		<div class="mt-8 rounded-2xl border border-dashed border-border bg-surface/50 p-12 text-center">
			<p class="text-muted-foreground">
				{canCreateStudents ? 'No students yet. Add your first student to begin.' : studentCreationBlockedMessage}
			</p>
		</div>
	{:else}
		<div class="table-wrap mt-6">
			<table class="w-full min-w-[760px] text-sm">
				<thead class="bg-surface text-left">
					<tr>
						<th class="label-mono px-4 py-3">
							<button
								onclick={() => onToggleSort('name')}
								class="inline-flex items-center gap-1 transition-colors hover:text-primary"
							>
								Name
								{#if sortBy === 'name'}
									<svg
										class="size-3"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
									>
										<path d={sortOrder === 'asc' ? 'm18 15-6-6-6 6' : 'm6 9 6 6 6-6'} />
									</svg>
								{/if}
							</button>
						</th>
						<th class="label-mono px-4 py-3">Gender</th>
						<th class="label-mono px-4 py-3">Class</th>
						<th class="label-mono px-4 py-3">Card</th>
						<th class="label-mono w-36 px-4 py-3 text-right">Actions</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-border">
					{#each paginatedStudents as s (s.id)}
						<tr>
							<td class="px-4 py-3">
								<button
									onclick={() => onOpenAttendance(s)}
									class="group flex min-w-0 items-center gap-2 text-left font-medium transition-colors hover:text-primary"
								>
									<span class="text-balance-safe">{s.name}</span>
									<svg
										class="size-3 opacity-0 transition-opacity group-hover:opacity-100"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2.5"
										stroke-linecap="round"
										stroke-linejoin="round"
									>
										<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
										<polyline points="15 3 21 3 21 9" />
										<line x1="10" y1="14" x2="21" y2="3" />
									</svg>
								</button>
							</td>
							<td class="px-4 py-3">
								<span class="rounded-pill border border-border bg-surface px-2 py-0.5 text-xs">
									{genderLabel(s.gender)}
								</span>
							</td>
							<td class="px-4 py-3">
								<span class="rounded-pill border border-border bg-surface px-2 py-0.5 text-xs">
									{s.classId || '—'}
								</span>
							</td>
							<td class="px-4 py-3 font-mono text-xs">
								{#if s.cardSerial}
									<span class="rounded-pill border border-border bg-surface px-2 py-1"
										>{s.cardSerial}</span
									>
								{:else}
									<span class="text-muted-foreground">—</span>
								{/if}
							</td>
							<td class="px-4 py-3 text-right">
								<div class="inline-flex gap-1">
									<a
										href={resolve(`/records?studentId=${s.id}`)}
										class="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface"
										title="View attendance records"
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
											<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
											<polyline points="14 2 14 8 20 8" />
											<line x1="16" y1="13" x2="8" y2="13" />
											<line x1="16" y1="17" x2="8" y2="17" />
											<polyline points="10 9 9 9 8 9" />
										</svg>
									</a>
									<button
										onclick={() => onOpenScan(s)}
										class="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface"
										title="Pair card"
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
									<button
										onclick={() => onOpenEdit(s)}
										class="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface"
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
									<button
										onclick={(event) => onDelete(event, s)}
										class="inline-flex size-8 items-center justify-center rounded-md border border-border bg-background text-destructive transition-colors hover:bg-surface"
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

<div class="fixed bottom-6 left-1/2 z-10 -translate-x-1/2">
	<Pagination currentPage={currentPage} totalPages={totalPages} onPageChange={onPageChange} />
</div>
