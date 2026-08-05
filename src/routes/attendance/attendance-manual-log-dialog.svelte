<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import type { Student, AttendanceType } from '$lib/db-rust';

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
	description={`Search by name to manually record attendance for ${selectedDateLabel}.`}
	onClose={() => (open = false)}
>
	<input
		placeholder="Search name..."
		bind:value={pickerQuery}
		class="w-full rounded-md border border-border bg-background px-4 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
	/>

	<ul class="max-h-[300px] divide-y divide-border overflow-y-auto rounded-xl border border-border">
		{#if pickerStudents.length === 0}
			<li class="py-10 text-center text-sm text-muted-foreground">
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
						class="flex w-full items-center justify-between px-4 py-3 text-left transition-colors hover:bg-surface disabled:cursor-not-allowed disabled:opacity-50"
					>
						<span>
							<span class="block font-medium">{student.name}</span>
							<span
								class="mt-0.5 block text-xs {status.tone === 'present'
									? 'text-green-700'
									: status.tone === 'absent'
										? 'text-red-700'
										: 'text-muted-foreground'}"
							>
								{status.label}
							</span>
						</span>
						<span
							class="label-mono text-xs font-bold {action === 'in'
								? 'text-primary'
								: 'text-destructive'}"
						>
							{action === 'in' ? 'RECORD' : 'MARK ABSENT'}
						</span>
					</button>
				</li>
			{/each}
		{/if}
	</ul>

	<div class="flex justify-end pt-2">
		<button
			onclick={() => (open = false)}
			class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
		>
			Close
		</button>
	</div>
</Dialog>
