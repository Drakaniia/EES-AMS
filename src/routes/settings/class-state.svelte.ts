import type { Class } from '$lib/features/settings/native';

/**
 * The Settings → Class record (spec §12.1, D15, D18).
 *
 * ## Read-only, on purpose
 *
 * D18 removes Settings → Classes add/edit/delete, leaving a single record. The
 * *data* is unchanged - `listClasses` still loads on every settings load, and
 * every class-aware screen reads the same rows - only the mutation path is gone.
 *
 * ## What was removed, and why it is not a loss of capability
 *
 * `saveClass` and `deleteClass` had exactly one caller between them: this state
 * object, driving a dialog that edited the single row the app already had. There
 * is no second class to add (D2: one class, twelve files) and no second row to
 * edit, so the CRUD was an editing surface for one record that the record was
 * already showing correctly.
 *
 * The one real cost, recorded here so it is not lost: a class record that is
 * *wrong* - a typo in the name, a room that changed - can no longer be corrected
 * from the UI. `create_class` / `update_class` / `delete_class` are still
 * registered in Rust, so restoring the affordance is a UI-only change; see the
 * Phase 7 report.
 */
class ClassState {
	// ── State ──────────────────────────────────────────────────────────────────
	classes = $state<Class[]>([]);

	/** The one class, when there is one. `null` on a fresh install. */
	primaryClass = $derived<Class | null>(this.classes[0] ?? null);
}

export const classState = new ClassState();
