<script lang="ts">
	import { Check, ChevronDown, Search } from 'lucide-svelte';
	import Dialog from './Dialog.svelte';
	import type { Student } from '$lib/api';

	type Props = {
		students: Student[];
		selectedId: string;
		classId?: string;
		placeholder?: string;
		onSelect?: (detail: { id: string }) => void;
	};

	let { students, selectedId, classId, placeholder = 'Select student', onSelect }: Props = $props();

	let open = $state(false);
	let query = $state('');

	let filteredStudents = $derived.by(() => {
		let result = students;

		if (classId) {
			result = result.filter((s) => s.classId === classId);
		}

		if (query.trim()) {
			const term = query.toLowerCase();
			result = result.filter((s) => s.name.toLowerCase().includes(term));
		}

		return result;
	});

	let selectedStudent = $derived(students.find((s) => s.id === selectedId));

	function handleSelect(id: string) {
		onSelect?.({ id });
		open = false;
		query = '';
	}

	function handleClear() {
		onSelect?.({ id: '' });
		open = false;
		query = '';
	}
</script>

<div class="w-full">
	<button
		type="button"
		onclick={() => (open = true)}
		aria-haspopup="dialog"
		aria-expanded={open}
		class="field flex items-center justify-between gap-2 text-left"
	>
		<span
			class={selectedStudent
				? 'truncate font-medium text-foreground'
				: 'truncate text-muted-foreground'}
		>
			{selectedStudent ? selectedStudent.name : placeholder}
		</span>
		<ChevronDown class="size-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
	</button>
</div>

<Dialog
	{open}
	title="Select student"
	description="Search the roster to filter records."
	onClose={() => (open = false)}
>
	<div class="space-y-3">
		<div class="search-field">
			<Search aria-hidden="true" />
			<input
				type="text"
				bind:value={query}
				placeholder="Search by name"
				class="field pl-[31px]"
				autocomplete="off"
			/>
		</div>

		<ul class="max-h-[320px] overflow-y-auto rounded-lg border border-border bg-card p-1">
			<li>
				<button
					onclick={handleClear}
					class="flex w-full items-center rounded-md px-2.5 py-2 text-left text-[13px] font-medium text-primary transition-colors hover:bg-surface"
				>
					All students
					<span class="ml-1.5 text-muted-foreground">— clear selection</span>
				</button>
			</li>
			{#if filteredStudents.length === 0}
				<li class="px-3 py-8 text-center text-[13px] text-muted-foreground">No students found.</li>
			{:else}
				{#each filteredStudents as s (s.id)}
					<li>
						<button
							onclick={() => handleSelect(s.id)}
							class="group flex w-full items-center justify-between gap-3 rounded-md px-2.5 py-2 text-left transition-colors hover:bg-surface"
						>
							<span class="min-w-0 flex-1 truncate text-[13px] font-medium">{s.name}</span>
							{#if s.id === selectedId}
								<Check
									class="size-3.5 shrink-0 text-primary"
									strokeWidth={2.4}
									aria-hidden="true"
								/>
							{/if}
						</button>
					</li>
				{/each}
			{/if}
		</ul>
	</div>

	<div class="flex justify-end border-t border-border pt-3">
		<button onclick={() => (open = false)} class="btn btn-secondary">Cancel</button>
	</div>
</Dialog>
