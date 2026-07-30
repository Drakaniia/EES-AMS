<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import LoadingBlock from '$lib/components/ui/LoadingBlock.svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';

	let toastMessage = $state('This page has moved to Attendance › Daily Overview.');
	let toastOk = $state(true);

	onMount(() => {
		setTimeout(() => {
			goto(resolve('/attendance/day-overview'), { replaceState: true });
		}, 1500);
	});
</script>

<svelte:head>
	<title>Redirecting — Attendance System</title>
</svelte:head>

<FeedbackToast message={toastMessage} ok={toastOk} />

<div class="px-4 py-5 md:px-8 lg:px-10">
	<LoadingBlock rows={2} label="Redirecting to Daily Overview" />
</div>
