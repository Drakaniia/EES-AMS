<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { Minus, Square, X } from 'lucide-svelte';

	let isMaximized = $state(false);
	let appWindow: ReturnType<typeof getCurrentWindow> | null = null;

	// Initialize window and check state
	$effect(() => {
		const initWindow = async () => {
			try {
				appWindow = getCurrentWindow();
				console.log('Window initialized:', appWindow);
				isMaximized = await appWindow.isMaximized();
				console.log('Window maximized state:', isMaximized);
			} catch (error) {
				console.error('Failed to initialize window:', error);
			}
		};
		initWindow();
	});

	async function minimize() {
		if (!appWindow) {
			console.error('Window not initialized');
			return;
		}
		try {
			console.log('Minimizing window...');
			await appWindow.minimize();
			console.log('Window minimized successfully');
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
			console.log('Toggling maximize...');
			const currentlyMaximized = await appWindow.isMaximized();
			console.log('Current maximized state:', currentlyMaximized);

			if (currentlyMaximized) {
				await appWindow.unmaximize();
				isMaximized = false;
				console.log('Window unmaximized');
			} else {
				await appWindow.maximize();
				isMaximized = true;
				console.log('Window maximized');
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
			console.log('Closing window...');
			await appWindow.close();
			console.log('Window closed successfully');
		} catch (error) {
			console.error('Failed to close window:', error);
		}
	}
</script>

<div class="title-bar" data-tauri-drag-region>
	<div class="title-bar-content" data-tauri-drag-region>
		<!-- App title or logo can go here -->
		<div class="app-title" data-tauri-drag-region>EES - Attendance Management System</div>
	</div>

	<div class="window-controls">
		<button class="window-control minimize" onclick={minimize} title="Minimize">
			<Minus size={14} />
		</button>
		<button
			class="window-control maximize"
			onclick={maximize}
			title={isMaximized ? 'Restore' : 'Maximize'}
		>
			<Square size={14} />
		</button>
		<button class="window-control close" onclick={close} title="Close">
			<X size={14} />
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
		transition: background-color 0.2s;
	}

	.window-control:hover {
		background: var(--color-surface);
	}

	.window-control.close:hover {
		background: var(--color-destructive);
		color: var(--color-destructive-foreground);
	}
</style>
