<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import { classDaysLabel as getDaysLabel } from '$lib/features/settings/class-schedule';
	import type { Class, Session } from '$lib/features/settings/native';

	let {
		open = $bindable(false),
		editingClass = $bindable<Class | null>(null),
		formClassName = $bindable(''),
		formRoom = $bindable(''),
		formDayStart = $bindable('08:00'),
		formDayEnd = $bindable('15:00'),
		formLateAfter = $bindable('08:45'),
		formSessions = $bindable<Session[]>([
			{ name: 'Full Day', startTime: '08:00', endTime: '15:00', lateAfter: '08:45' }
		]),
		formDays = $bindable<number[]>([1, 2, 3, 4, 5]),
		sessionMode = $bindable<'single' | 'morning-afternoon' | 'custom'>('single'),
		onsave
	}: {
		open?: boolean;
		editingClass?: Class | null;
		formClassName?: string;
		formRoom?: string;
		formDayStart?: string;
		formDayEnd?: string;
		formLateAfter?: string;
		formSessions?: Session[];
		formDays?: number[];
		sessionMode?: 'single' | 'morning-afternoon' | 'custom';
		onsave?: (e: SubmitEvent) => void;
	} = $props();

	function handleSessionModeChange(mode: typeof sessionMode) {
		sessionMode = mode;
		if (mode === 'single') {
			formSessions = [
				{
					name: 'Full Day',
					startTime: formDayStart,
					endTime: formDayEnd,
					lateAfter: formLateAfter
				}
			];
		} else if (mode === 'morning-afternoon') {
			formSessions = [
				{ name: 'Morning', startTime: '07:30', endTime: '11:30', lateAfter: '07:45' },
				{ name: 'Afternoon', startTime: '13:00', endTime: '17:00', lateAfter: '13:15' }
			];
		}
	}

	function addSession() {
		formSessions = [
			...formSessions,
			{
				name: `Session ${formSessions.length + 1}`,
				startTime: '08:00',
				endTime: '12:00',
				lateAfter: '08:15'
			}
		];
	}

	function removeSession(index: number) {
		formSessions = formSessions.filter((_, i) => i !== index);
	}
</script>

<Dialog
	{open}
	title={editingClass ? 'Edit Class' : 'Add New Class'}
	description="Define the schedule for this specific grade or section."
	onClose={() => (open = false)}
>
	<form onsubmit={onsave} class="space-y-4">
		<div class="grid grid-cols-2 gap-4">
			<div class="space-y-1.5">
				<label for="className" class="label-mono">Class Name</label>
				<input
					id="className"
					bind:value={formClassName}
					placeholder=""
					required
					class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
			<div class="space-y-1.5">
				<label for="room" class="label-mono"
					>Room <span class="font-normal text-muted-foreground">(optional)</span></label
				>
				<input
					id="room"
					bind:value={formRoom}
					placeholder=" "
					class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
				/>
			</div>
		</div>

		<!-- Days of Week Selector -->
		<fieldset class="space-y-1.5">
			<legend class="label-mono flex items-center justify-between">
				<span>Scheduled Days</span>
				<span class="text-[10px] font-medium tracking-wider text-muted-foreground uppercase">
					{getDaysLabel(formDays)}
				</span>
			</legend>
			<div class="flex justify-between gap-1">
				{#each ['S', 'M', 'T', 'W', 'T', 'F', 'S'] as day, i (i)}
					<button
						type="button"
						onclick={() => {
							if (formDays.includes(i)) {
								formDays = formDays.filter((d) => d !== i);
							} else {
								formDays = [...formDays, i].sort();
							}
						}}
						class="flex size-9 items-center justify-center rounded-md border text-xs font-semibold transition-colors
							{formDays.includes(i)
							? 'border-primary bg-primary text-primary-foreground'
							: 'border-border bg-background hover:bg-surface'}"
					>
						{day}{i === 4 ? 'H' : ''}
					</button>
				{/each}
			</div>
		</fieldset>

		<!-- Session Mode Selector -->
		<fieldset class="space-y-1.5">
			<legend class="label-mono">Session Mode</legend>
			<div class="flex gap-2">
				<button
					type="button"
					onclick={() => handleSessionModeChange('single')}
					class="flex-1 rounded-md border px-3 py-2 text-sm transition-colors {sessionMode ===
					'single'
						? 'border-primary bg-primary text-primary-foreground'
						: 'border-border bg-background hover:bg-surface'}"
				>
					Single Day
				</button>
				<button
					type="button"
					onclick={() => handleSessionModeChange('morning-afternoon')}
					class="flex-1 rounded-md border px-3 py-2 text-sm transition-colors {sessionMode ===
					'morning-afternoon'
						? 'border-primary bg-primary text-primary-foreground'
						: 'border-border bg-background hover:bg-surface'}"
				>
					Morning & Afternoon
				</button>
				<button
					type="button"
					onclick={() => (sessionMode = 'custom')}
					class="flex-1 rounded-md border px-3 py-2 text-sm transition-colors {sessionMode ===
					'custom'
						? 'border-primary bg-primary text-primary-foreground'
						: 'border-border bg-background hover:bg-surface'}"
				>
					Custom
				</button>
			</div>
		</fieldset>

		<!-- Sessions List -->
		<div class="space-y-3">
			<div class="flex items-center justify-between">
				<h4 class="label-mono text-xs text-muted-foreground uppercase">Sessions</h4>
				{#if sessionMode === 'custom'}
					<button
						type="button"
						onclick={addSession}
						class="text-xs font-medium text-accent hover:underline"
					>
						+ Add Session
					</button>
				{/if}
			</div>

			<div class="max-h-64 space-y-4 overflow-y-auto pr-1">
				{#each formSessions as session, i (i)}
					<div class="relative space-y-3 rounded-xl border border-border p-4">
						{#if sessionMode === 'custom' && formSessions.length > 1}
							<button
								type="button"
								aria-label="Remove session {i + 1}"
								onclick={() => removeSession(i)}
								class="absolute top-3 right-3 text-muted-foreground hover:text-destructive"
							>
								<svg
									class="size-4"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
								>
									<path d="M18 6L6 18M6 6l12 12" />
								</svg>
							</button>
						{/if}

						<div class="grid grid-cols-2 gap-4">
							<div class="space-y-1">
								<label class="text-xs font-medium text-muted-foreground">
									Session Name
									<input
										bind:value={session.name}
										placeholder="e.g. Morning"
										required
										readonly={sessionMode !== 'custom'}
										class="mt-1 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</label>
							</div>
							<div class="space-y-1">
								<label class="text-xs font-medium text-muted-foreground">
									Late After
									<input
										type="time"
										bind:value={session.lateAfter}
										required
										class="mt-1 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</label>
							</div>
						</div>

						<div class="grid grid-cols-2 gap-4">
							<div class="space-y-1">
								<label class="text-xs font-medium text-muted-foreground">
									Start Time
									<input
										type="time"
										bind:value={session.startTime}
										required
										class="mt-1 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</label>
							</div>
							<div class="space-y-1">
								<label class="text-xs font-medium text-muted-foreground">
									End Time
									<input
										type="time"
										bind:value={session.endTime}
										required
										class="mt-1 w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm focus:ring-2 focus:ring-primary focus:outline-none"
									/>
								</label>
							</div>
						</div>
					</div>
				{/each}
			</div>
		</div>

		<div class="flex justify-end gap-2 pt-2">
			<button
				type="button"
				onclick={() => (open = false)}
				class="rounded-md border border-border px-4 py-2 text-sm transition-colors hover:bg-surface"
			>
				Cancel
			</button>
			<button
				type="submit"
				class="rounded-pill bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent"
			>
				{editingClass ? 'Save Changes' : 'Create Class'}
			</button>
		</div>
	</form>
</Dialog>
