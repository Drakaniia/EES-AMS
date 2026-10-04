/**
 * The SF2 workbook lifecycle: writing a class's working copy from the bundled DepEd
 * template, and rewriting it from a fresh draft.
 *
 * The port of `src-tauri/src/sf2/template/{template_create,template_update}.rs`.
 * `template_ops.rs` had nothing left to port: it emitted `sf2-progress` events around
 * one call, and there is no Excel process left to report on.
 */

export { createWorkbookFromTemplate } from './create';
export { updateWorkbookSettings } from './update';
export { importSf2WorkbookFromFile, stageImportSource, validateSf2WorkbookImport } from './import';
