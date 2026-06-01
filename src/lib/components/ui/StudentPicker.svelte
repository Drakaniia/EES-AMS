<script lang="ts">
	import Dialog from './Dialog.svelte';
	import type { Student } from '$lib/db-rust';

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
		class="flex h-10 w-full items-center justify-between rounded-md border border-border bg-background px-3 text-left text-sm transition-colors hover:bg-surface focus:ring-2 focus:ring-primary focus:outline-none"
	>
		<span class={selectedStudent ? 'font-medium text-foreground' : 'text-muted-foreground'}>
			{selectedStudent ? selectedStudent.name : placeholder}
		</span>
		<svg
			class="size-4 text-muted-foreground"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<path d="m6 9 6 6 6-6" />
		</svg>
	</button>
</div>

<Dialog
	{open}
	title="Select Student"
	description="Search for a student to filter records."
	onClose={() => (open = false)}
>
	<div class="space-y-4">
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
				bind:value={query}
				placeholder="Search by name..."
				class="h-10 w-full rounded-md border border-border bg-background pr-4 pl-10 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				autocomplete="off"
			/>
		</div>

		<ul
			class="max-h-[300px] divide-y divide-border overflow-y-auto rounded-xl border border-border bg-surface/30"
		>
			<li>
				<button
					onclick={handleClear}
					class="flex w-full items-center px-4 py-3 text-left text-sm font-medium text-primary transition-colors hover:bg-surface"
				>
					All Students (Clear selection)
				</button>
			</li>
			{#if filteredStudents.length === 0}
				<li class="py-10 text-center text-sm text-muted-foreground">No students found.</li>
			{:else}
				{#each filteredStudents as s (s.id)}
					<li>
						<button
							onclick={() => handleSelect(s.id)}
							class="group flex w-full items-center justify-between px-4 py-3 text-left transition-colors hover:bg-surface"
						>
							<div class="min-w-0 flex-1">
								<div class="truncate font-medium group-hover:text-primary">{s.name}</div>
							</div>
							{#if s.id === selectedId}
								<svg
									class="size-4 text-primary"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="3"
								>
									<polyline points="20 6 9 17 4 12" />
								</svg>
							{/if}
						</button>
					</li>
				{/each}
			{/if}
		</ul>
	</div>

	<div class="flex justify-end gap-2 border-t border-border pt-4">
		<button
			onclick={() => (open = false)}
			class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
		>
			Cancel
		</button>
	</div>
</Dialog>
