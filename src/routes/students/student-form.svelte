<script lang="ts">
	import type { Student, StudentGender } from '$lib/api';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import { genderOptions, entryModeTabs, type EntryMode } from './student-state.svelte';

	let {
		open,
		editing,
		entryMode,
		formName,
		formGender,
		bulkMaleStudentNames,
		bulkFemaleStudentNames,
		assignedClassLabel,
		canCreateStudents,
		savingStudent,
		bulkMaleNames,
		bulkFemaleNames,
		bulkStudentCount,
		onClose,
		onSubmit,
		onSetEntryMode,
		onFormNameChange,
		onFormGenderChange,
		onBulkMaleChange,
		onBulkFemaleChange
	}: {
		open: boolean;
		editing: Student | null;
		entryMode: EntryMode;
		formName: string;
		formGender: StudentGender;
		bulkMaleStudentNames: string;
		bulkFemaleStudentNames: string;
		assignedClassLabel: string;
		canCreateStudents: boolean;
		savingStudent: boolean;
		bulkMaleNames: string[];
		bulkFemaleNames: string[];
		bulkStudentCount: number;
		onClose: () => void;
		onSubmit: (e: SubmitEvent) => void;
		onSetEntryMode: (mode: EntryMode) => void;
		onFormNameChange: (value: string) => void;
		onFormGenderChange: (value: StudentGender) => void;
		onBulkMaleChange: (value: string) => void;
		onBulkFemaleChange: (value: string) => void;
	} = $props();
</script>

<Dialog
	{open}
	title={editing ? 'Edit student' : 'Add student'}
	description={editing
		? 'Update the student profile and class assignment.'
		: 'Add one student manually, or paste the class list in one pass.'}
	maxWidth="3xl"
	{onClose}
>
	<form onsubmit={onSubmit} class="space-y-4" data-mode={entryMode}>
		{#if !editing}
			<div class="segmented w-full" role="group" aria-label="Student entry mode">
				{#each entryModeTabs as tab (tab.value)}
					<button
						type="button"
						aria-pressed={entryMode === tab.value}
						onclick={() => onSetEntryMode(tab.value)}
						class="segmented-item flex-1"
					>
						{tab.label}
					</button>
				{/each}
			</div>
		{/if}

		<div
			class="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface/60 px-3 py-2"
		>
			<span class="text-[11.5px] font-medium text-muted-foreground">Class / Section</span>
			<span class="text-[12.5px] font-medium">{assignedClassLabel}</span>
		</div>

		{#key entryMode}
			{#if !editing && entryMode === 'bulk'}
				<div class="space-y-2.5">
					<div class="flex items-baseline justify-between gap-3">
						<p class="text-[12.5px] font-medium">Student names</p>
						<span class="text-[11.5px] text-muted-foreground tabular-nums">
							{bulkStudentCount}
							{bulkStudentCount === 1 ? 'student' : 'students'}
						</span>
					</div>

					<div class="grid gap-3 sm:grid-cols-2">
						<div class="space-y-1.5">
							<div class="flex items-center justify-between gap-2">
								<label
									for="bulk-male-students"
									class="text-[11.5px] font-medium text-muted-foreground"
								>
									Male
								</label>
								<span class="text-[11px] text-muted-foreground tabular-nums"
									>{bulkMaleNames.length}</span
								>
							</div>
							<textarea
								id="bulk-male-students"
								value={bulkMaleStudentNames}
								oninput={(e) => onBulkMaleChange((e.currentTarget as HTMLTextAreaElement).value)}
								rows="9"
								placeholder="CRUZ, JUAN M.&#10;REYES, MARCO A."
								class="textarea-field min-h-56 font-mono text-[12px]"
							></textarea>
						</div>
						<div class="space-y-1.5">
							<div class="flex items-center justify-between gap-2">
								<label
									for="bulk-female-students"
									class="text-[11.5px] font-medium text-muted-foreground"
								>
									Female
								</label>
								<span class="text-[11px] text-muted-foreground tabular-nums"
									>{bulkFemaleNames.length}</span
								>
							</div>
							<textarea
								id="bulk-female-students"
								value={bulkFemaleStudentNames}
								oninput={(e) => onBulkFemaleChange((e.currentTarget as HTMLTextAreaElement).value)}
								rows="9"
								placeholder="DELA CRUZ, MARIA R.&#10;SANTOS, ANA L."
								class="textarea-field min-h-56 font-mono text-[12px]"
							></textarea>
						</div>
					</div>
					<p class="text-[11.5px] leading-5 text-muted-foreground">
						Each line creates one student in the matching SF2 roster block.
					</p>
				</div>
			{:else}
				<div class="space-y-3">
					<div class="space-y-1.5">
						<span class="text-[11.5px] font-medium text-muted-foreground">Gender</span>
						<div class="segmented w-full">
							{#each genderOptions as option (option.value)}
								<button
									type="button"
									onclick={() => onFormGenderChange(option.value)}
									aria-pressed={formGender === option.value}
									class="segmented-item flex-1"
								>
									{option.label}
								</button>
							{/each}
						</div>
					</div>
					<div class="space-y-1.5">
						<label for="field-name" class="text-[11.5px] font-medium text-muted-foreground">
							Full name
						</label>
						<input
							id="field-name"
							value={formName}
							oninput={(e) => onFormNameChange((e.currentTarget as HTMLInputElement).value)}
							required
							placeholder="LASTNAME, FIRSTNAME M."
							class="field"
						/>
					</div>
				</div>
			{/if}
		{/key}

		<div class="flex flex-col-reverse gap-2 border-t border-border pt-4 sm:flex-row sm:justify-end">
			<button type="button" onclick={onClose} class="btn btn-secondary">Cancel</button>
			<button
				type="submit"
				disabled={savingStudent ||
					(!editing && (!canCreateStudents || (entryMode === 'bulk' && bulkStudentCount === 0)))}
				class="btn btn-primary min-w-28"
			>
				{#if savingStudent}
					Saving…
				{:else if editing}
					Save changes
				{:else if entryMode === 'bulk'}
					Add {bulkStudentCount || ''} students
				{:else}
					Add student
				{/if}
			</button>
		</div>
	</form>
</Dialog>
