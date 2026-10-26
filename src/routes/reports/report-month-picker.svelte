<script lang="ts">
	import { Check } from 'lucide-svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import { SF2_SCHOOL_MONTHS } from '$lib/features/settings/sf2-workbook';

	type Props = {
		open: boolean;
		currentMonth: string;
		activeClassId: string;
		onSelect?: (monthValue: string) => void;
		onClose?: () => void;
	};

	let { open, currentMonth, activeClassId, onSelect, onClose }: Props = $props();
</script>

<Dialog {open} title="Switch SF2 report month" {onClose}>
	<div class="grid grid-cols-2 gap-1.5">
		{#each SF2_SCHOOL_MONTHS as month (month.value)}
			<button
				type="button"
				onclick={() => {
					if (!onSelect || !month.value || month.value === currentMonth || !activeClassId) return;
					onSelect(month.value);
				}}
				disabled={!activeClassId}
				class="control-ring flex items-center justify-between gap-2 rounded-md border px-3 py-2 text-left text-[12.5px] font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 {month.value ===
				currentMonth
					? 'border-primary/40 bg-primary/[0.055] text-foreground'
					: 'border-border bg-card text-foreground hover:bg-surface'}"
			>
				<span>{month.label}</span>
				{#if month.value === currentMonth}
					<Check class="size-3.5 shrink-0 text-primary" strokeWidth={2.4} aria-hidden="true" />
				{/if}
			</button>
		{/each}
	</div>
</Dialog>
