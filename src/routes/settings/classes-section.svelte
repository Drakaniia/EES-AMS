<script lang="ts">
	import { classState, settingsState } from './settings-state.svelte';
	import { classDaysLabel as getDaysLabel } from '$lib/features/settings/class-schedule';
	import ClassDialog from './class-dialog.svelte';
	import DeleteConfirmDialog from './delete-confirm-dialog.svelte';
</script>

<section class="overflow-hidden rounded-2xl border border-border bg-card">
	<div class="flex items-center justify-between p-6 pb-4">
		<h3 class="text-lg font-medium">Classes & Schedule</h3>
		<button
			onclick={() =>
				classState.openAddClass(
					settingsState.defaultDayStart,
					settingsState.defaultDayEnd,
					settingsState.defaultLateAfter
				)}
			disabled={classState.classes.length > 0}
			title={classState.classes.length > 0
				? 'Only one class is supported for this teacher'
				: 'Add class'}
			class="inline-flex items-center gap-2 rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent disabled:cursor-not-allowed disabled:opacity-60"
		>
			<svg
				class="size-4"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M12 5v14M5 12h14" />
			</svg>
			{classState.classes.length > 0 ? 'One Class Only' : 'Add Class'}
		</button>
	</div>

	<div class="divide-y divide-border border-t border-border pt-5">
		{#if classState.classes.length === 0}
			<div class="p-12 text-center text-sm text-muted-foreground">
				No classes configured. Add a class to start tracking attendance.
			</div>
		{:else}
			{#each classState.classes as c (c.id)}
				<div class="flex items-center justify-between p-6 transition-colors hover:bg-surface">
					<div class="space-y-1">
						<div class="flex items-center gap-3">
							<div class="font-medium">{c.name}</div>
							{#if c.days}
								<span
									class="rounded-md bg-accent/10 px-2 py-0.5 text-[10px] font-bold tracking-wide text-accent uppercase"
								>
									{getDaysLabel(c.days)}
								</span>
							{/if}
						</div>
						<div class="label-mono flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
							{#if c.room}
								<span>Room {c.room}</span>
							{/if}
							{#if c.sessions && c.sessions.length > 0}
								{#each c.sessions as s (s.name)}
									<span class="inline-flex items-center gap-1">
										<span class="font-medium text-foreground">{s.name}:</span>
										{s.startTime}–{s.endTime}
									</span>
								{/each}
							{:else}
								<span>{c.dayStart} – {c.dayEnd}</span>
								<span class="text-accent">Late after {c.lateAfter}</span>
							{/if}
						</div>
					</div>
					<div class="flex gap-2">
						<button
							onclick={() => classState.openEditClass(c)}
							class="inline-flex size-9 items-center justify-center rounded-md border border-border bg-background transition-colors hover:bg-surface"
							title="Edit class"
						>
							<svg
								class="size-4"
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
							onclick={(event) => classState.onDeleteClass(event, c.id)}
							class="inline-flex size-9 items-center justify-center rounded-md border border-border bg-background text-destructive transition-colors hover:bg-surface"
							title="Delete class"
						>
							<svg
								class="size-4"
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
				</div>
			{/each}
		{/if}
	</div>
</section>

<ClassDialog
	bind:open={classState.classDialogOpen}
	bind:editingClass={classState.editingClass}
	bind:formClassName={classState.formClassName}
	bind:formRoom={classState.formRoom}
	bind:formDayStart={classState.formDayStart}
	bind:formDayEnd={classState.formDayEnd}
	bind:formLateAfter={classState.formLateAfter}
	bind:formSessions={classState.formSessions}
	bind:formDays={classState.formDays}
	bind:sessionMode={classState.sessionMode}
	onsave={(e) => classState.onSaveClass(e)}
/>

<DeleteConfirmDialog
	open={classState.deleteTarget !== null}
	bind:target={classState.deleteTarget}
	onconfirm={() => classState.confirmDeleteClass()}
/>
