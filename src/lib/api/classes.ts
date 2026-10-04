/**
 * `$lib/api/classes.ts` → `$lib/db/repos/classes`.
 *
 * Read-only, as the old bridge documented: the per-month model is one class by
 * design (spec D2, D15), so `listClasses` / `getClass` are the whole surface. The
 * repo's `Class` is the `Class` in `$lib/types`, which is what the bridge's
 * hand-written field mapping used to hand back.
 *
 * The repo also carries `createClass` / `updateClass` / `deleteClass`. They are
 * deliberately not re-exported: nothing in the UI called them through the bridge,
 * and exporting them would put a class CRUD screen one import away.
 */

export { getClass, listClasses } from '$lib/db/repos/classes';

export type { Class } from '$lib/types';
