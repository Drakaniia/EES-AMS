<script lang="ts">
	import { onMount } from 'svelte';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { Minus, Square, X } from 'lucide-svelte';

	let isMaximized = $state(false);
	let appWindow: ReturnType<typeof getCurrentWindow> | null = null;

	onMount(() => {
		appWindow = getCurrentWindow();
		appWindow
			.isMaximized()
			.then((maximized) => {
				isMaximized = maximized;
			})
			.catch((error) => {
				console.error('Failed to initialize window:', error);
			});
	});

	async function minimize() {
		if (!appWindow) {
			console.error('Window not initialized');
			return;
		}
		try {
			await appWindow.minimize();
		} catch (error) {
			console.error('Failed to minimize window:', error);
		}
	}

	async function maximize() {
		if (!appWindow) {
			console.error('Window not initialized');
			return;
		}
		try {
			const currentlyMaximized = await appWindow.isMaximized();

			if (currentlyMaximized) {
				await appWindow.unmaximize();
				isMaximized = false;
			} else {
				await appWindow.maximize();
				isMaximized = true;
			}
		} catch (error) {
			console.error('Failed to toggle maximize:', error);
		}
	}

	async function close() {
		if (!appWindow) {
			console.error('Window not initialized');
			return;
		}
		try {
			await appWindow.close();
		} catch (error) {
			console.error('Failed to close window:', error);
		}
	}
</script>

<div class="title-bar" data-tauri-drag-region>
	<div class="title-bar-content" data-tauri-drag-region>
		<!-- App title or logo can go here -->
		<div class="app-title" data-tauri-drag-region>EES Attendance Management System</div>
	</div>

	<div class="window-controls" aria-label="Window controls">
		<button
			class="window-control minimize"
			onclick={minimize}
			title="Minimize"
			aria-label="Minimize"
		>
			<Minus size={14} aria-hidden="true" />
		</button>
		<button
			class="window-control maximize"
			onclick={maximize}
			title={isMaximized ? 'Restore' : 'Maximize'}
			aria-label={isMaximized ? 'Restore window' : 'Maximize window'}
		>
			<Square size={14} aria-hidden="true" />
		</button>
		<button class="window-control close" onclick={close} title="Close" aria-label="Close window">
			<X size={14} aria-hidden="true" />
		</button>
	</div>
</div>

<style>
	.title-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		height: 32px;
		background: var(--color-background);
		border-bottom: 1px solid var(--color-border);
		user-select: none;
		-webkit-app-region: drag;
	}

	.title-bar-content {
		flex: 1;
		display: flex;
		align-items: center;
		padding-left: 12px;
		-webkit-app-region: drag;
	}

	.app-title {
		font-size: 14px;
		font-weight: 500;
		color: var(--color-muted-foreground);
		-webkit-app-region: drag;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.window-controls {
		display: flex;
		-webkit-app-region: no-drag;
	}

	.window-control {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 46px;
		height: 32px;
		border: none;
		background: transparent;
		cursor: pointer;
		color: var(--color-muted-foreground);
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
	}

	.window-control:hover {
		background: var(--color-surface);
	}

	.window-control.close:hover {
		background: var(--color-destructive);
		color: var(--color-destructive-foreground);
	}
</style>
