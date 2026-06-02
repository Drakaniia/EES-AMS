<script lang="ts">
	import AppShell from '$lib/components/layout/AppShell.svelte';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
</script>

<svelte:head>
	<title>Teachers - Attendance System</title>
	<meta name="description" content="Teacher management availability." />
</svelte:head>

<AppShell>
	<PageHeader
		category="Teachers"
		title="Teacher management is not connected yet"
		description="This build exposes student, class, attendance, settings, and SF2 commands. Teacher entities exist in Rust, but no Tauri teacher commands are registered for a production UI."
	/>

	<div class="px-4 py-5 md:px-8 lg:px-10">
		<EmptyState
			tone="warning"
			title="No teacher workflow is available in this frontend contract"
			description="Teacher records cannot be managed until teacher list, create, update, delete, and card-pairing commands are exposed by the Tauri backend."
		/>
	</div>
</AppShell>
