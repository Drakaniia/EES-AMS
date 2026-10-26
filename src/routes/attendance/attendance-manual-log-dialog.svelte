<script lang="ts">
	import { Search } from 'lucide-svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import type { Student, AttendanceType } from '$lib/api';

	let {
		open = $bindable(false),
		pickerQuery = $bindable(''),
		pickerStudents = [] as Student[],
		selectedDateLabel = '',
		selectedClassId = '',
		isProcessing = false,
		dateLoading = false,
		getNextAttendanceType = () => null as AttendanceType | null,
		getStudentStatus = () => ({ label: '', tone: '' }) as { label: string; tone: string },
		markStudent = async () => {},
		onMarkAbsent = () => {}
	}: {
		open: boolean;
		pickerQuery: string;
		pickerStudents: Student[];
		selectedDateLabel: string;
		selectedClassId: string;
		isProcessing: boolean;
		dateLoading: boolean;
		getNextAttendanceType: (student: Student) => AttendanceType | null;
		getStudentStatus: (student: Student) => { label: string; tone: string };
		markStudent: (
			student: Student,
			action: AttendanceType | null,
			closePicker: boolean
		) => Promise<void>;
		onMarkAbsent: (student: Student) => void;
	} = $props();
</script>

<Dialog
	{open}
	title="Manual log"
	description={`Search by name to record attendance for ${selectedDateLabel}.`}
	onClose={() => (open = false)}
>
	<div class="search-field">
		<Search aria-hidden="true" />
		<input placeholder="Search name" bind:value={pickerQuery} class="field pl-[31px]" />
	</div>

	<ul class="max-h-[320px] overflow-y-auto rounded-lg border border-border bg-card p-1">
		{#if pickerStudents.length === 0}
			<li class="px-3 py-9 text-center text-[13px] text-muted-foreground">
				No names found {selectedClassId ? 'in this class' : ''}.
			</li>
		{:else}
			{#each pickerStudents as student (student.id)}
				{@const action = getNextAttendanceType(student)}
				{@const status = getStudentStatus(student)}
				<li
					oncontextmenu={(e) => {
						e.preventDefault();
						onMarkAbsent(student);
					}}
				>
					<button
						disabled={isProcessing || dateLoading}
						onclick={() => markStudent(student, action, true)}
						class="flex w-full items-center justify-between gap-3 rounded-md px-2.5 py-2 text-left transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
					>
						<span class="min-w-0">
							<span class="block truncate text-[13px] font-medium">{student.name}</span>
							<span
								class="mt-0.5 block text-[11px] {status.tone === 'present'
									? 'text-emerald-700'
									: status.tone === 'absent'
										? 'text-red-700'
										: 'text-muted-foreground'}"
							>
								{status.label}
							</span>
						</span>
						<span
							class="shrink-0 text-[11px] font-semibold {action === 'in'
								? 'text-primary'
								: 'text-destructive'}"
						>
							{action === 'in' ? 'Record' : 'Mark absent'}
						</span>
					</button>
				</li>
			{/each}
		{/if}
	</ul>

	<div class="flex justify-end pt-1">
		<button onclick={() => (open = false)} class="btn btn-secondary">Close</button>
	</div>
</Dialog>
