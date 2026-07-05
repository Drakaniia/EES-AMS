<script lang="ts">
	import type { Student, StudentGender } from '$lib/db-rust';
	import {
		genderOptions,
		entryModeTabs,
		parseStudentNames,
		type EntryMode,
	} from './student-state.svelte';

	let {
		open,
		editing,
		entryMode,
		entryModeDirection,
		formName,
		formGender,
		formCardSerial,
		formClassId,
		bulkMaleStudentNames,
		bulkFemaleStudentNames,
		assignedClassLabel,
		sf2Template,
		assignedClass,
		canCreateStudents,
		studentCreationBlockedMessage,
		savingStudent,
		bulkMaleNames,
		bulkFemaleNames,
		bulkStudentCount,
		onClose,
		onSubmit,
		onSetEntryMode,
		onFormNameChange,
		onFormGenderChange,
		onFormCardSerialChange,
		onBulkMaleChange,
		onBulkFemaleChange,
	}: {
		open: boolean;
		editing: Student | null;
		entryMode: EntryMode;
		entryModeDirection: number;
		formName: string;
		formGender: StudentGender;
		formCardSerial: string;
		formClassId: string;
		bulkMaleStudentNames: string;
		bulkFemaleStudentNames: string;
		assignedClassLabel: string;
		sf2Template: unknown;
		assignedClass: { id: string } | null;
		canCreateStudents: boolean;
		studentCreationBlockedMessage: string;
		savingStudent: boolean;
		bulkMaleNames: string[];
		bulkFemaleNames: string[];
		bulkStudentCount: number;
		onClose: () => void;
		onSubmit: (e: SubmitEvent) => void;
		onSetEntryMode: (mode: EntryMode) => void;
		onFormNameChange: (value: string) => void;
		onFormGenderChange: (value: StudentGender) => void;
		onFormCardSerialChange: (value: string) => void;
		onBulkMaleChange: (value: string) => void;
		onBulkFemaleChange: (value: string) => void;
	} = $props();
</script>

{#if open}
	<div
		class="fixed inset-0 z-40 bg-black/50"
		role="presentation"
		onclick={onClose}
		onkeydown={(e) => e.key === 'Escape' && onClose()}
	></div>

	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="dialog-title"
	>
		<div class="w-full max-w-3xl rounded-2xl border border-border bg-background">
			<div class="border-b border-border px-6 pt-6 pb-5">
				<h2 id="dialog-title" class="text-lg font-semibold">
					{editing ? 'Edit student' : 'Add student'}
				</h2>
				<p class="mt-1 text-sm text-muted-foreground">
					{editing
						? 'Update the student profile and class assignment.'
						: 'Add one student manually or paste a class list in one pass.'}
				</p>
			</div>

			<form onsubmit={onSubmit} class="space-y-5 p-6">
				{#if !editing}
					<div
						class="add-student-entry-tabs relative grid overflow-hidden rounded-lg border border-border bg-surface p-1 sm:grid-cols-2"
						data-mode={entryMode}
						role="tablist"
						aria-label="Student entry mode"
					>
						<span class="add-student-tab-indicator" aria-hidden="true"></span>
						{#each entryModeTabs as tab (tab.value)}
							<button
								id={`add-student-${tab.value}-tab`}
								type="button"
								role="tab"
								aria-selected={entryMode === tab.value}
								aria-controls={`add-student-${tab.value}-panel`}
								onclick={() => onSetEntryMode(tab.value)}
								class="relative z-10 rounded-md px-4 py-2 text-sm font-medium transition-colors {entryMode ===
									tab.value
									? 'text-foreground'
									: 'text-muted-foreground hover:text-foreground'}"
							>
								{tab.label}
							</button>
						{/each}
					</div>
				{/if}

				<div class="space-y-1.5">
					<label for="field-class" class="label-mono">Class / Section</label>
					<div
						id="field-class"
						class="w-full rounded-md border border-border bg-surface px-3 py-2 text-sm font-medium"
					>
						{assignedClassLabel}
					</div>
				</div>

				{#key entryMode}
					<div
						id={!editing ? `add-student-${entryMode}-panel` : undefined}
						role={!editing ? 'tabpanel' : undefined}
						aria-labelledby={!editing ? `add-student-${entryMode}-tab` : undefined}
					>
						{#if !editing && entryMode === 'bulk'}
							<div class="space-y-2">
								<div class="flex items-center justify-between gap-3">
									<div class="label-mono">Student names</div>
									<span class="font-mono text-xs text-muted-foreground">
										{bulkStudentCount}
										{bulkStudentCount === 1 ? 'student' : 'students'}
									</span>
								</div>
								<div class="grid gap-4 sm:grid-cols-2">
									<div class="space-y-2">
										<div class="flex items-center justify-between gap-2">
											<label for="bulk-male-students" class="label-mono">Male</label>
											<span class="font-mono text-xs text-muted-foreground"
												>{bulkMaleNames.length}</span
											>
										</div>
										<textarea
											id="bulk-male-students"
											value={bulkMaleStudentNames}
											oninput={(e) => onBulkMaleChange((e.currentTarget as HTMLTextAreaElement).value)}
											rows="10"
											placeholder="Cruz, Juan&#10;Reyes, Marco"
											class="min-h-64 w-full resize-y rounded-md border border-border bg-background px-3 py-3 text-sm leading-6 focus:ring-2 focus:ring-primary focus:outline-none"
										></textarea>
									</div>
									<div class="space-y-2">
										<div class="flex items-center justify-between gap-2">
											<label for="bulk-female-students" class="label-mono">Female</label>
											<span class="font-mono text-xs text-muted-foreground"
												>{bulkFemaleNames.length}</span
											>
										</div>
										<textarea
											id="bulk-female-students"
											value={bulkFemaleStudentNames}
											oninput={(e) => onBulkFemaleChange((e.currentTarget as HTMLTextAreaElement).value)}
											rows="10"
											placeholder="Dela Cruz, Maria&#10;Santos, Ana"
											class="min-h-64 w-full resize-y rounded-md border border-border bg-background px-3 py-3 text-sm leading-6 focus:ring-2 focus:ring-primary focus:outline-none"
										></textarea>
									</div>
								</div>
								<p class="text-xs text-muted-foreground">
									Each new line creates one student in the matching SF2 roster block.
								</p>
							</div>
						{:else}
							<div class="space-y-2">
								<div class="label-mono">Gender</div>
								<div class="grid rounded-lg border border-border bg-surface p-1 sm:grid-cols-2">
									{#each genderOptions as option (option.value)}
										<button
											type="button"
											onclick={() => onFormGenderChange(option.value)}
											aria-pressed={formGender === option.value}
											class="rounded-md px-4 py-2 text-sm font-medium transition-colors {formGender ===
												option.value
												? 'bg-background text-foreground shadow-sm'
												: 'text-muted-foreground hover:text-foreground'}"
										>
											{option.label}
										</button>
									{/each}
								</div>
							</div>
							<div class="grid gap-4 sm:grid-cols-2">
								<div class="space-y-1.5">
									<label for="field-name" class="label-mono">Full name</label>
									<input
										id="field-name"
										value={formName}
										oninput={(e) => onFormNameChange((e.currentTarget as HTMLInputElement).value)}
										required
										placeholder="Student full name"
										class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</div>
								<div class="space-y-1.5">
									<label for="field-card" class="label-mono">Card serial (optional)</label>
									<input
										id="field-card"
										value={formCardSerial}
										oninput={(e) => onFormCardSerialChange((e.currentTarget as HTMLInputElement).value)}
										placeholder="Pair later"
										class="w-full rounded-md border border-border bg-background px-3 py-2 font-mono text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</div>
							</div>
						{/if}
					</div>
				{/key}

				<div
					class="flex flex-col-reverse gap-2 border-t border-border pt-5 sm:flex-row sm:justify-end"
				>
					<button
						type="button"
						onclick={onClose}
						class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
					>
						Cancel
					</button>
					<button
						type="submit"
						disabled={savingStudent ||
							(!editing &&
								(!canCreateStudents || (entryMode === 'bulk' && bulkStudentCount === 0)))}
						class="rounded-pill bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
					>
						{#if savingStudent}
							Saving...
						{:else if editing}
							Save Changes
						{:else if entryMode === 'bulk'}
							Add {bulkStudentCount || ''} Students
						{:else}
							Add Student
						{/if}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<style>
	.add-student-tab-indicator {
		position: absolute;
		inset: 0.25rem auto 0.25rem 0.25rem;
		width: calc(50% - 0.25rem);
		border-radius: 0.375rem;
		background: var(--color-background);
		border: 1px solid var(--color-border);
	}

	.add-student-entry-tabs[data-mode='bulk'] .add-student-tab-indicator {
		transform: translateX(100%);
	}
</style>
