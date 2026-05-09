<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import type { ServerInfo } from '$lib/types';

	let serverInfo = $state<ServerInfo | null>(null);
	let copied = $state(false);

	onMount(async () => {
		try {
			serverInfo = await invoke<ServerInfo>('get_server_info');
		} catch (e) {
			console.error('Failed to get server info:', e);
		}
	});

	async function copyUrl() {
		if (serverInfo) {
			await navigator.clipboard.writeText(serverInfo.url);
			copied = true;
			setTimeout(() => (copied = false), 2000);
		}
	}
</script>

{#if serverInfo}
	<div class="server-info">
		<div class="info-header">
			<svg
				class="icon"
				xmlns="http://www.w3.org/2000/svg"
				width="16"
				height="16"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<circle cx="12" cy="12" r="10"></circle>
				<path d="M12 16v-4"></path>
				<path d="M12 8h.01"></path>
			</svg>
			<span class="label">Server Running</span>
		</div>

		<div class="url-container">
			<code class="url">{serverInfo.url}</code>
			<button class="copy-btn" onclick={copyUrl} aria-label="Copy URL">
				{#if copied}
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<polyline points="20 6 9 17 4 12"></polyline>
					</svg>
				{:else}
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="16"
						height="16"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
						<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
					</svg>
				{/if}
			</button>
		</div>

		<p class="help-text">
			Open this URL on your Android phone (Chrome) to use NFC scanning. Make sure both devices are
			on the same network.
		</p>
	</div>
{/if}

<style>
	.server-info {
		background: var(--surface, #f0ebe1);
		border: 1px solid var(--border, #292524);
		border-radius: 12px;
		padding: 16px;
		margin-bottom: 24px;
	}

	.info-header {
		display: flex;
		align-items: center;
		gap: 8px;
		margin-bottom: 12px;
	}

	.icon {
		color: var(--primary, #ea580c);
	}

	.label {
		font-family: 'JetBrains Mono', monospace;
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.url-container {
		display: flex;
		align-items: center;
		gap: 8px;
		background: white;
		border: 1px solid var(--border, #292524);
		border-radius: 8px;
		padding: 8px 12px;
		margin-bottom: 8px;
	}

	.url {
		flex: 1;
		font-family: 'JetBrains Mono', monospace;
		font-size: 14px;
		color: var(--text-primary, #111827);
	}

	.copy-btn {
		background: transparent;
		border: none;
		cursor: pointer;
		padding: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-secondary, #4b5563);
		transition: color 0.2s;
	}

	.copy-btn:hover {
		color: var(--primary, #ea580c);
	}

	.help-text {
		font-size: 13px;
		color: var(--text-secondary, #4b5563);
		line-height: 1.5;
		margin: 0;
	}
</style>
