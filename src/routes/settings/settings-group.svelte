<script lang="ts">
	import type { Snippet } from 'svelte';

	/** An inset grouped card, the unit macOS System Settings builds a pane from. */
	let { children }: { children: Snippet } = $props();
</script>

<div class="overflow-hidden rounded-xl border border-border bg-card">
	<div class="divide-y divide-border">
		{@render children()}
	</div>
</div>
