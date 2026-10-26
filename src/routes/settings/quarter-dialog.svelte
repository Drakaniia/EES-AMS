<script lang="ts">
	import Dialog from '$lib/components/ui/Dialog.svelte';

	let {
		open = $bindable(false),
		quarter = $bindable('1st Quarter'),
		q1Start = $bindable(''),
		q1End = $bindable(''),
		q2Start = $bindable(''),
		q2End = $bindable(''),
		q3Start = $bindable(''),
		q3End = $bindable('')
	}: {
		open?: boolean;
		quarter?: string;
		q1Start?: string;
		q1End?: string;
		q2Start?: string;
		q2End?: string;
		q3Start?: string;
		q3End?: string;
	} = $props();
</script>

<Dialog
	{open}
	title="School Year Quarters"
	description="Set the current quarter and the dates each period covers."
	onClose={() => (open = false)}
>
	<div class="space-y-6">
		<div class="space-y-2">
			<label for="currentQuarter" class="text-[11.5px] font-medium text-muted-foreground"
				>Active quarter</label
			>
			<select id="currentQuarter" bind:value={quarter} class="select-field">
				<option value="1st Quarter">1st Quarter</option>
				<option value="2nd Quarter">2nd Quarter</option>
				<option value="3rd Quarter">3rd Quarter</option>
			</select>
		</div>

		<div class="space-y-4">
			<h3 class="text-[12.5px] font-semibold text-foreground">Quarter dates</h3>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q1Start" class="text-xs font-medium text-muted-foreground">Q1 Start</label>
					<input id="q1Start" type="date" bind:value={q1Start} class="field" />
				</div>
				<div class="space-y-1">
					<label for="q1End" class="text-xs font-medium text-muted-foreground">Q1 End</label>
					<input id="q1End" type="date" bind:value={q1End} class="field" />
				</div>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q2Start" class="text-xs font-medium text-muted-foreground">Q2 Start</label>
					<input id="q2Start" type="date" bind:value={q2Start} class="field" />
				</div>
				<div class="space-y-1">
					<label for="q2End" class="text-xs font-medium text-muted-foreground">Q2 End</label>
					<input id="q2End" type="date" bind:value={q2End} class="field" />
				</div>
			</div>

			<div class="grid grid-cols-2 gap-4">
				<div class="space-y-1">
					<label for="q3Start" class="text-xs font-medium text-muted-foreground">Q3 Start</label>
					<input id="q3Start" type="date" bind:value={q3Start} class="field" />
				</div>
				<div class="space-y-1">
					<label for="q3End" class="text-xs font-medium text-muted-foreground">Q3 End</label>
					<input id="q3End" type="date" bind:value={q3End} class="field" />
				</div>
			</div>
		</div>

		<div class="flex justify-end border-t border-border pt-3">
			<button onclick={() => (open = false)} class="btn btn-primary">Done</button>
		</div>
	</div>
</Dialog>
