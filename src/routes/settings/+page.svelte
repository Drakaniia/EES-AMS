<script lang="ts">
	import { onMount } from 'svelte';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { settingsState } from './settings-state.svelte';

	import ClassesSection from './classes-section.svelte';
	import BackupSection from './backup-section.svelte';
	import Sf2Section from './sf2-section.svelte';
	import GlobalConfigForm from './global-config-form.svelte';
	import RestoreBackupDialog from './restore-backup-dialog.svelte';

	onMount(() => {
		settingsState.init();
	});
</script>

<svelte:head>
	<title>Settings — Attendance System</title>
	<meta name="description" content="Manage your classes and system configuration." />
</svelte:head>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
	<div class="min-h-0 flex-1 overflow-auto">
		{#if settingsStore.loading}
			<div class="px-6 py-12 text-sm text-muted-foreground md:px-12">Loading…</div>
		{:else if settingsStore.error}
			<div class="px-6 py-12 text-sm text-destructive md:px-12">
				Error: {settingsStore.error}
				<button onclick={() => settingsState.reload()} class="ml-2 underline">Retry</button>
			</div>
		{:else}
			<div class="grid gap-6 px-6 py-6 md:px-12 lg:grid-cols-12">
				<!-- ── Left column ───────────────────────────────────────────── -->
				<div class="flex flex-col gap-6 lg:col-span-8">
					<ClassesSection />
					<Sf2Section />
					<BackupSection />
				</div>

				<!-- ── Right column ──────────────────────────────────────────── -->
				<div class="space-y-6 lg:col-span-4">
					<GlobalConfigForm />
				</div>
			</div>
		{/if}
	</div>
</div>

<RestoreBackupDialog />
<FeedbackToast
	message={settingsState.toastMessage}
	ok={settingsState.toastOk}
	onClose={() => (settingsState.toastMessage = null)}
/>
