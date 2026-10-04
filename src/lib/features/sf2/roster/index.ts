/**
 * The SF2 roster: reading a workbook's learners, giving each student a row, and
 * re-pointing a workbook at a class's current roster. The module is split by the job
 * each file does — `parser` (slot arithmetic), `helpers` (shared lookups),
 * `learner-sync` (names to students), `sync` (class to workbook), `internal` (the two
 * workbook-mutating branches), `analysis` (workbook to business view),
 * `formula-marks` (the marks a roster change writes).
 */

export { readWorkbookAnalysis } from './analysis';
export {
	bundledTemplateTotalRows,
	expandedRosterSlots,
	rejectDuplicateRosterNames,
	rosterExpansionNeeded,
	rosterNameMarks,
	studentMappingsFromRosterAssignments,
	templateOwnsRoster,
	templateRosterAssignments,
	templateRosterSlots,
	uniqueNormalizedName,
	type TemplateRosterAssignment,
	type TemplateRosterSlot
} from './parser';
export type { Sf2TotalRows } from '$lib/features/excel/formula-marks';
export { clearUnusedLearnerMarks, findOrCreateClass, genderCounts } from './helpers';
export {
	rosterStudentsForDraft,
	syncWorkbookLearnerMappings,
	syncWorkbookLearnerMappingsWithOld,
	type DraftStudents,
	type WorkbookLearnerSync
} from './learner-sync';
export { rosterSyncFormulaMarks, type RosterSyncFormulaMarks } from './formula-marks';
export {
	syncLatestWorkbookRosterForClass,
	syncTemplateRosterFromClass,
	syncWorkbookRosterForClass
} from './sync';
export { syncBundledTemplateRoster, syncImportedWorkbookRoster } from './internal';
