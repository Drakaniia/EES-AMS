<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import FeedbackToast from '$lib/components/ui/FeedbackToast.svelte';
	import { settingsStore } from '$lib/stores/settings.svelte';
	import { settingsState } from './settings-state.svelte';

	import ClassesSection from './classes-section.svelte';
	import BackupSection from './backup-section.svelte';
	import Sf2Section from './sf2-section.svelte';
	import GlobalConfigForm from './global-config-form.svelte';
	import UpdateSection from './update-section.svelte';
	import RestoreBackupDialog from './restore-backup-dialog.svelte';
	import BrandingSection from './branding-section.svelte';

	onMount(() => {
		settingsState.init();
	});

	// Deep links from the command palette (#settings-<section>). Sections render
	// asynchronously after settings load, so retry briefly until the target
	// element exists.
	$effect(() => {
		const hash = page.url.hash;
		if (!hash) return;
		const id = decodeURIComponent(hash.slice(1));
		let attempts = 0;
		let timer: ReturnType<typeof setTimeout> | undefined;
		const tryScroll = () => {
			const el = document.getElementById(id);
			if (el) {
				el.scrollIntoView({ behavior: 'smooth', block: 'start' });
				return;
			}
			if (attempts++ < 20) timer = setTimeout(tryScroll, 100);
		};
		tryScroll();
		return () => {
			if (timer) clearTimeout(timer);
		};
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
					<div id="settings-classes" class="scroll-mt-6">
						<ClassesSection />
					</div>
					<div id="settings-sf2" class="scroll-mt-6">
						<Sf2Section />
					</div>
					<div id="settings-backup" class="scroll-mt-6">
						<BackupSection />
					</div>
				</div>
				<!-- ── Right column ──────────────────────────────────────────── -->
				<div class="space-y-6 lg:col-span-4">
					<div id="settings-branding" class="scroll-mt-6">
						<BrandingSection />
					</div>
					<div id="settings-global" class="scroll-mt-6">
						<GlobalConfigForm />
					</div>
					<div id="settings-update" class="scroll-mt-6">
						<UpdateSection />
					</div>
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
