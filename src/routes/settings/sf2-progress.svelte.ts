import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/**
 * Manages SF2 workbook progress overlay state and Tauri event listening.
 *
 * Handles the progress lifecycle: setup → cycle → cleanup → hide.
 * Exposes reactive $state fields consumable from Svelte templates.
 */
export class Sf2ProgressManager {
	/** The active task name ('import' | 'create' | 'update' | ''). */
	task = $state('');
	/** Current progress step (1-based). */
	current = $state(0);
	/** Total progress steps. */
	total = $state(0);
	/** The progress overlay is visible. */
	visible = $state(false);
	/** The current display message shown in the overlay. */
	displayMessage = $state('');

	private unlisten: UnlistenFn | null = null;
	private lastBackendMsg = $state('');
	private lastBackendTime = $state(0);
	private cycleTimer: ReturnType<typeof setInterval> | null = null;

	/** Friendly fallback messages that cycle when the backend is quiet. */
	private static readonly PROGRESS_MESSAGES: Record<string, string[]> = {
		import: [
			'Reading the SF2 workbook…',
			'Validating student data…',
			'Creating student mappings…',
			'Still working on it…',
			'Just a moment longer…'
		],
		create: [
			'Setting up the bundled template…',
			'Copying the workbook from the template…',
			'Writing student names into the workbook…',
			'Configuring attendance date columns…',
			'Saving the working copy…',
			'Almost there…',
			'Finalizing your SF2 workbook…'
		],
		update: ['Updating workbook settings…', 'Reconfiguring calendar…', 'Writing changes…']
	};

	private cycleIndex = 0;

	async setup(task: string) {
		this.cleanup();
		this.task = task;
		this.current = 0;
		this.total = 0;
		this.visible = true;
		this.displayMessage = '';
		this.lastBackendMsg = '';
		this.lastBackendTime = 0;
		this.cycleIndex = 0;

		try {
			this.unlisten = await listen<{
				task: string;
				current: number;
				total: number;
				message: string;
			}>('sf2-progress', (event) => {
				if (event.payload.task === this.task) {
					this.current = event.payload.current;
					this.total = event.payload.total;
					if (event.payload.message) {
						this.lastBackendMsg = event.payload.message;
						this.lastBackendTime = Date.now();
					}
					this.updateMessage();
				}
			});
		} catch {
			// Listener setup failed — continue without it
		}

		this.startCycle();
	}

	private startCycle() {
		this.stopCycle();
		this.updateMessage();
		this.cycleTimer = setInterval(() => {
			const now = Date.now();
			if (now - this.lastBackendTime > 3000) {
				const messages =
					Sf2ProgressManager.PROGRESS_MESSAGES[this.task] ??
					Sf2ProgressManager.PROGRESS_MESSAGES.import;
				this.cycleIndex = (this.cycleIndex + 1) % messages.length;
			}
			this.updateMessage();
		}, 2500);
	}

	private stopCycle() {
		if (this.cycleTimer !== null) {
			clearInterval(this.cycleTimer);
			this.cycleTimer = null;
		}
	}

	private updateMessage() {
		if (this.lastBackendMsg && Date.now() - this.lastBackendTime < 4000) {
			this.displayMessage = this.lastBackendMsg;
			return;
		}

		if (this.current > 0 && this.total > 0) {
			const stepMessages: Record<string, Record<number, string>> = {
				import: {
					1: 'Analyzing workbook structure…',
					2: 'Finding class for imported workbook…',
					3: 'Processing student data…',
					4: 'Validating learner roster…',
					5: 'Creating date mappings…',
					6: 'Creating working copy…',
					7: 'Finalizing workbook…'
				},
				create: {
					1: 'Creating SF2 working workbook…',
					2: 'Finalizing workbook…'
				}
			};
			const taskMessages = stepMessages[this.task];
			const stepMessage = taskMessages?.[this.current];
			if (stepMessage) {
				this.displayMessage = stepMessage;
				return;
			}
		}

		const messages =
			Sf2ProgressManager.PROGRESS_MESSAGES[this.task] ??
			Sf2ProgressManager.PROGRESS_MESSAGES.import;
		this.displayMessage = messages[this.cycleIndex % messages.length];
	}

	cleanup() {
		this.stopCycle();
		if (this.unlisten) {
			this.unlisten();
			this.unlisten = null;
		}
	}

	hide() {
		this.cleanup();
		this.visible = false;
		this.task = '';
	}
}
