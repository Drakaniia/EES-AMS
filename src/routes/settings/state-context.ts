/**
 * Cross-cutting services injected into sub-state classes.
 * Implemented by SettingsPageState (the orchestrator).
 */
export interface Ctx {
	toast(msg: string, ok?: boolean): void;
	reload(): Promise<void>;
	/** Whether Global Settings has unsaved edits (update restart guard). */
	hasUnsavedGlobalSettings(): boolean;
}
