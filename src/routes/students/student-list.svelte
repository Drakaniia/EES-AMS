<script lang="ts">
	import { resolve } from '$app/paths';
	import { FileText, Pencil, Search, Trash2, ChevronUp, ChevronDown } from 'lucide-svelte';
	import Pagination from '$lib/components/ui/Pagination.svelte';
	import type { Student } from '$lib/api';
	import { genderLabel } from './student-state.svelte';

	let {
		students,
		paginatedStudents,
		searchTerms,
		genderFilter,
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
		onGenderFilterChange,
		onToggleSort,
		onPageChange,
		onOpenAttendance,
		onOpenEdit,
		onDelete,
		availableHeight = $bindable(0)
	}: {
		students: Student[];
		paginatedStudents: Student[];
		searchTerms: string;
		genderFilter: 'all' | 'male' | 'female';
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
		onGenderFilterChange: (value: 'all' | 'male' | 'female') => void;
		onToggleSort: (field: 'name' | 'date') => void;
		onPageChange: (page: number) => void;
		onOpenAttendance: (student: Student) => void;
		onOpenEdit: (student: Student) => void;
		onDelete: (event: MouseEvent, student: Student) => void;
		availableHeight?: number;
	} = $props();
</script>

<div class="page-frame flex min-h-0 flex-1 flex-col pb-24">
	<!-- One row of tools. The roster is the page; this strip just narrows it. -->
	<section class="flex flex-wrap items-center gap-2" aria-label="Roster filters">
		<div class="search-field min-w-60 flex-1">
			<Search aria-hidden="true" />
			<input
				type="text"
				value={searchTerms}
				oninput={(e) => onSearchChange((e.currentTarget as HTMLInputElement).value)}
				placeholder="Search by name"
				aria-label="Search students"
				class="field pl-[31px]"
			/>
		</div>

		<div class="segmented" role="group" aria-label="Gender filter">
			<button
				type="button"
				aria-pressed={genderFilter === 'all'}
				onclick={() => onGenderFilterChange('all')}
				class="segmented-item"
			>
				All
			</button>
			<button
				type="button"
				aria-pressed={genderFilter === 'male'}
				onclick={() => onGenderFilterChange('male')}
				class="segmented-item"
			>
				Male
			</button>
			<button
				type="button"
				aria-pressed={genderFilter === 'female'}
				onclick={() => onGenderFilterChange('female')}
				class="segmented-item"
			>
				Female
			</button>
		</div>

		<span class="chip">
			{assignedClassLabel}
		</span>

		<div class="ml-auto flex items-baseline gap-2 text-[11.5px] text-muted-foreground">
			<span class="text-[13px] font-semibold text-foreground tabular-nums">
				{filteredStudents.length}
			</span>
			<span>of {students.length} students</span>
			<span class="text-border" aria-hidden="true">|</span>
			<span class="tabular-nums">{maleStudentCount} M</span>
			<span class="tabular-nums">{femaleStudentCount} F</span>
		</div>
	</section>

	<section class="flex min-h-0 flex-1 flex-col pt-4" bind:clientHeight={availableHeight}>
		{#if students.length === 0}
			<div
				class="flex flex-1 flex-col items-center justify-center rounded-2xl border border-dashed border-border bg-card/40 px-6 py-12 text-center"
			>
				<p class="text-[14px] font-medium">
					{canCreateStudents ? 'No students in this class yet' : 'This class list is empty'}
				</p>
				<p class="mt-1 max-w-md text-[12.5px] leading-5 text-muted-foreground">
					{canCreateStudents
						? 'Add the first student, or paste the whole class list at once, to start recording attendance.'
						: studentCreationBlockedMessage}
				</p>
			</div>
		{:else if filteredStudents.length === 0}
			<div
				class="flex flex-1 flex-col items-center justify-center rounded-2xl border border-dashed border-border bg-card/40 px-6 py-12 text-center"
			>
				<p class="text-[14px] font-medium">No students match these filters</p>
				<p class="mt-1 text-[12.5px] text-muted-foreground">
					Clear the search or set gender back to All.
				</p>
			</div>
		{:else}
			<div class="table-wrap min-h-0 flex-1">
				<table class="min-w-[720px] text-[13px]" aria-label="Student roster">
					<caption class="sr-only">Student roster with search, filter, and sort controls</caption>
					<thead class="text-left">
						<tr>
							<th class="px-4 py-2.5">
								<button
									onclick={() => onToggleSort('name')}
									class="inline-flex items-center gap-1 font-semibold transition-colors hover:text-foreground"
								>
									Name
									{#if sortBy === 'name'}
										{#if sortOrder === 'asc'}
											<ChevronUp class="size-3" strokeWidth={2.4} aria-hidden="true" />
										{:else}
											<ChevronDown class="size-3" strokeWidth={2.4} aria-hidden="true" />
										{/if}
									{/if}
								</button>
							</th>
							<th class="w-28 px-4 py-2.5">Gender</th>
							<th class="w-32 px-4 py-2.5">Class</th>
							<th class="w-32 px-4 py-2.5 text-right">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each paginatedStudents as s (s.id)}
							<tr>
								<td class="px-4 py-2.5">
									<button
										onclick={() => onOpenAttendance(s)}
										class="text-balance-safe text-left font-medium transition-colors hover:text-primary"
										title="View this student's attendance"
									>
										{s.name}
									</button>
								</td>
								<td class="px-4 py-2.5">
									<span class="chip">{genderLabel(s.gender)}</span>
								</td>
								<td class="px-4 py-2.5">
									<span class="chip">{s.classId || '—'}</span>
								</td>
								<td class="px-4 py-2.5">
									<div class="flex items-center justify-end gap-0.5">
										<a
											href={resolve(`/attendance/logs?studentId=${s.id}`)}
											class="icon-btn size-7"
											title="View attendance records"
											aria-label={`View attendance records for ${s.name}`}
										>
											<FileText class="size-3.5" strokeWidth={1.9} aria-hidden="true" />
										</a>
										<button
											onclick={() => onOpenEdit(s)}
											class="icon-btn size-7"
											title="Edit student"
											aria-label={`Edit ${s.name}`}
										>
											<Pencil class="size-3.5" strokeWidth={1.9} aria-hidden="true" />
										</button>
										<button
											onclick={(event) => onDelete(event, s)}
											class="icon-btn size-7 hover:bg-destructive/10 hover:text-destructive"
											title="Delete student"
											aria-label={`Delete ${s.name}`}
										>
											<Trash2 class="size-3.5" strokeWidth={1.9} aria-hidden="true" />
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
</div>

<div class="fixed bottom-6 left-1/2 z-10 -translate-x-1/2">
	<Pagination {currentPage} {totalPages} {onPageChange} />
</div>
