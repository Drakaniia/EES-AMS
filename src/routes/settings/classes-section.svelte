<script lang="ts">
	import { classState } from './settings-state.svelte';
	import { classDaysLabel as getDaysLabel } from '$lib/features/settings/class-schedule';
	import SettingsSection from './settings-section.svelte';
	import SettingsGroup from './settings-group.svelte';
	import SettingRow from './setting-row.svelte';
</script>

<!--
	A read-only record of the one class, per spec §12.1 / D15.

	D15 keeps the `classes` table and every class-aware path: the schema is not
	locked to one class, and the per-month workbook model is built on
	`active_class_id`. What D18 removes is the *Settings* CRUD for it — the add,
	edit and delete controls that only ever operated on the single row the app
	already had, behind a dialog, on a screen nobody opens.

	So this states the record and nothing else.
-->

<SettingsSection
	title="Class"
	description="The class this app keeps attendance for. Every SF2 workbook belongs to it."
>
	{#if classState.classes.length === 0}
		<SettingsGroup>
			<SettingRow
				label="No class on record yet"
				description="One is created for you the first time you add students."
			/>
		</SettingsGroup>
	{:else}
		{#each classState.classes as c (c.id)}
			<SettingsGroup>
				<SettingRow label="Name">
					<span class="text-sm">{c.name}</span>
				</SettingRow>

				{#if c.room}
					<SettingRow label="Room">
						<span class="font-mono text-sm">{c.room}</span>
					</SettingRow>
				{/if}

				{#if c.days}
					<SettingRow label="School days">
						<span class="text-sm">{getDaysLabel(c.days)}</span>
					</SettingRow>
				{/if}

				{#if c.sessions && c.sessions.length > 0}
					{#each c.sessions as s (s.name)}
						<SettingRow label={s.name}>
							<span class="font-mono text-sm">{s.startTime}–{s.endTime}</span>
						</SettingRow>
					{/each}
				{:else}
					<SettingRow label="School hours">
						<span class="font-mono text-sm">{c.dayStart} – {c.dayEnd}</span>
					</SettingRow>
				{/if}

				<SettingRow label="Late after">
					<span class="font-mono text-sm text-accent">{c.lateAfter}</span>
				</SettingRow>
			</SettingsGroup>
		{/each}
	{/if}
</SettingsSection>
