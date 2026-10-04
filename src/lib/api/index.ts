// Barrel re-exports — every import that used to resolve through '$lib/api'
// resolves here, with the same names and the same signatures.
//
// Deliberately absent, and each one is a call site to delete rather than a
// behaviour to reimplement (migration spec D10/D14/D15):
//
// - `killAllExcelProcesses` is a throwing stub here (spec D14) — see './sf2'.
// - The whole of the SF2 startup self-heal: `healCurrentMonthWorkbook`,
//   `onSf2HealOutcome`, `SF2_HEAL_OUTCOME_EVENT` (spec D15).
// - `chooseBackupSyncFolder`, `clearBackupSyncFolder`,
//   `connectGoogleDriveBackup`, `disconnectGoogleDriveBackup`,
//   `uploadLatestBackupToGoogleDrive`, `chooseRestoreDatabaseFile` (spec D10).
export * from './students';
export * from './classes';
export * from './events';
export * from './settings';
export * from './backup';
export * from './sf2';
export * from './sf2-months';

// Re-export types from the shared types module
export type {
	AttendanceType,
	Session,
	UpdateEventRequest,
	Sf2CloseDaySummary,
	BackupSummary,
	BackupStatus
} from '$lib/types';
