<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';

	onMount(() => {
		goto(resolve('/'), { replaceState: true });
	});
</script>

<svelte:head>
	<title>Dashboard - Attendance System</title>
</svelte:head>

<AppShell>
	<div class="px-4 py-5 md:px-8 lg:px-10">
		<LoadingBlock rows={2} label="Opening dashboard" />
	</div>
</AppShell>
