<script lang="ts">
	import { classState } from './settings-state.svelte';
	import { classDaysLabel as getDaysLabel } from '$lib/features/settings/class-schedule';
</script>

<!--
	A read-only record of the one class, per spec §12.1 / D15.

	D15 keeps the `classes` table and every class-aware path: the schema is not
	locked to one class, and the per-month workbook model is built on
	`active_class_id`. What D18 removes is the *Settings* CRUD for it — the add,
	edit and delete controls that only ever operated on the single row the app
	already had, behind a dialog, on a screen nobody opens.

	So this states the record and nothing else. The data it shows is the same data
	every other screen reads; the absence of buttons here is the decision, and it
	is a decision with a cost — see the note under the record.
-->
<section class="overflow-hidden rounded-2xl border border-border bg-card">
	<div class="p-6 pb-4">
		<h3 class="text-lg font-medium">Class</h3>
		<p class="mt-1 text-sm text-muted-foreground">
			The class this app keeps attendance for. Every SF2 workbook belongs to it.
		</p>
	</div>

	<div class="border-t border-border pt-5">
		{#if classState.classes.length === 0}
			<div class="p-12 text-center text-sm text-muted-foreground">
				No class on record yet. One is created for you the first time you add students.
			</div>
		{:else}
			{#each classState.classes as c (c.id)}
				<div class="p-6">
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
				</div>
			{/each}
		{/if}
	</div>
</section>
