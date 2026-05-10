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
			const result: string = await invoke('download_and_install');
			console.log(result);
			// The app will restart automatically after successful installation
		} catch (error) {
			downloadError = error as string;
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
		
		let message = `Version ${info.version} is available`;
		if (info.currentVersion) {
			message += ` (you have ${info.currentVersion})`;
		}
		
		if (info.notes) {
			// Truncate notes if too long
			const notes = info.notes.length > 100 
				? info.notes.substring(0, 100) + '...' 
				: info.notes;
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
		duration={0} // Don't auto-hide update notifications
		closable={!isDownloading}
		actionText={isDownloading ? 'Downloading...' : 'Download & Install'}
		action={isDownloading ? undefined : downloadAndInstall}
		on:close={dismissUpdate}
	/>
{/if}

{#if downloadError}
	<Toast
		type="error"
		title="Update Failed"
		message={downloadError}
		duration={10000}
		closable={true}
	/>
{/if}
