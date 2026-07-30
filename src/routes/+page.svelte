<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';

	onMount(() => {
		goto(resolve('/reports'), { replaceState: true });
	});
</script>

<svelte:head>
	<title>Redirecting - Attendance System</title>
</svelte:head>

<div class="px-4 py-5 md:px-8 lg:px-10">
	<LoadingBlock rows={2} label="Opening SF2 Reports" />
</div>
