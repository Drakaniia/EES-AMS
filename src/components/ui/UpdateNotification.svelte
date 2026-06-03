<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import Toast from './Toast.svelte';

	interface UpdateInfo {
		available: boolean;
		version?: string;
		notes?: string;
		pubDate?: string;
		currentVersion: string;
	}

	let updateInfo = $state<UpdateInfo | null>(null);
	let showUpdateToast = $state(false);
	let isDownloading = $state(false);
	let downloadError = $state('');
	let installMessage = $state('');

	onMount(async () => {
		await checkForUpdates();
	});

	async function checkForUpdates() {
		try {
			const result: UpdateInfo = await invoke('check_for_updates');
			updateInfo = result;

			if (result.available) {
				showUpdateToast = true;
			}
		} catch (error) {
			console.error('Failed to check for updates:', error);
		}
	}

	async function downloadAndInstall() {
		if (!updateInfo?.available) return;

		isDownloading = true;
		downloadError = '';

		try {
			installMessage = await invoke<string>('download_and_install');
		} catch (error) {
			downloadError = error instanceof Error ? error.message : String(error);
			console.error('Update failed:', error);
		} finally {
			isDownloading = false;
		}
	}

	function dismissUpdate() {
		showUpdateToast = false;
	}

	function formatUpdateMessage(info: UpdateInfo): string {
		if (!info.version) return 'An update is available';

		let message = `Version ${info.version} is ready to install`;
		if (info.currentVersion) {
			message += ` (current ${info.currentVersion})`;
		}

		if (info.notes) {
			const notes = info.notes.length > 100 ? info.notes.substring(0, 100) + '...' : info.notes;
			message += `\n\n${notes}`;
		}

		return message;
	}
</script>

{#if showUpdateToast && updateInfo}
	<Toast
		type="update"
		title="Update Available"
		message={formatUpdateMessage(updateInfo)}
		duration={0}
		closable={!isDownloading}
		actionText={isDownloading ? 'Downloading...' : 'Download & Install'}
		actionDisabled={isDownloading}
		action={isDownloading ? undefined : downloadAndInstall}
		onClose={dismissUpdate}
	/>
{/if}

{#if installMessage}
	<Toast
		type="success"
		title="Update Installed"
		message={installMessage}
		duration={10000}
		closable={true}
		onClose={() => (installMessage = '')}
	/>
{/if}

{#if downloadError}
	<Toast
		type="error"
		title="Update Failed"
		message={downloadError}
		duration={10000}
		closable={true}
		onClose={() => (downloadError = '')}
	/>
{/if}
