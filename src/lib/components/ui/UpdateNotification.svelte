<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import Toast from './Toast.svelte';
	import { updateStore } from '$lib/stores/update.svelte';

	let showUpdateToast = $state(false);

	function isTauriRuntime() {
		return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
	}

	onMount(() => {
		if (!isTauriRuntime()) return;
		void initToast();
	});

	async function initToast() {
		await updateStore.init();
		if (updateStore.stagedVersion) {
			// Reminder that a previously downloaded update is still pending
			showUpdateToast = true;
		} else if (updateStore.status === 'available' && updateStore.updateInfo) {
			showUpdateToast = true;
		}
	}

	function goToSettings() {
		showUpdateToast = false;
		goto(resolve('/settings'));
	}

	function dismissUpdate() {
		showUpdateToast = false;
	}

	function toastTitle(): string {
		return updateStore.stagedVersion ? 'Update Ready' : 'Update Available';
	}

	function toastMessage(): string {
		if (updateStore.stagedVersion) {
			return `Version ${updateStore.stagedVersion} is downloaded. Restart to apply the update.`;
		}
		const info = updateStore.updateInfo;
		if (!info) return 'An update is available';
		let message = `Version ${info.version ?? 'X'} is available`;
		if (info.currentVersion) {
			message += ` (current v${info.currentVersion})`;
		}
		if (info.notes) {
			const notes = info.notes.length > 100 ? info.notes.substring(0, 100) + '...' : info.notes;
			message += `\n\n${notes}`;
		}
		return message;
	}
</script>

{#if showUpdateToast}
	<Toast
		type="update"
		title={toastTitle()}
		message={toastMessage()}
		duration={0}
		closable={true}
		actionText="Go to Settings"
		action={goToSettings}
		onClose={dismissUpdate}
	/>
{/if}
