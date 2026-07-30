<script lang="ts">
	import AttendanceTabBar from '$lib/components/layout/AttendanceTabBar.svelte';

	let { children } = $props();
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
	<AttendanceTabBar />
	<div class="min-h-0 flex-1 overflow-auto">
		{@render children()}
	</div>
</div>
