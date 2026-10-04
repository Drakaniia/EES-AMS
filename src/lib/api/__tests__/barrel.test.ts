import { describe, expect, it } from 'vitest';
import * as api from '$lib/api';
import { useApiFixture } from './fixture';

/**
 * The contract this layer exists to hold: the UI's call sites do not change, so
 * every name `$lib/api` exported is still exported here with the same shape,
 * and every name the migration spec retires is *not* — a missing export that
 * compiles today is a missing deletion someone finds at runtime.
 */

useApiFixture();

/** Old `db-rust` export → whether it survived the migration. */
const SURVIVED = [
	// students
	'listStudents',
	'getStudent',
	'findStudentByCard',
	'saveStudent',
	'createStudents',
	'deleteStudent',
	'uid',
	// classes
	'listClasses',
	'getClass',
	// events
	'listEvents',
	'listEventsForDate',
	'listEventsForStudent',
	'lastEventForStudent',
	'addEvent',
	'addEvents',
	'updateEvent',
	'deleteEvent',
	'deleteEvents',
	'listAttendanceAudit',
	// settings and audit
	'listAuditEvents',
	'clearAuditEvents',
	'getSettings',
	'saveSettings',
	// backup
	'exportAll',
	'exportDatabase',
	'exportJsonWithFolder',
	'exportCsvWithFolder',
	'importAll',
	'wipeAll',
	'getBackupStatus',
	'createBackupNow',
	'createWorkbooksBackupNow',
	'listBackups',
	'openBackupFolder',
	'chooseRestoreBackup',
	'restoreBackup',
	// sf2
	'getSf2WorkbookSettings',
	'updateSf2WorkbookSettings',
	'getSf2ExportReadiness',
	'getSf2ExportPreview',
	'syncSf2Attendance',
	'toggleSf2PreviewAttendance',
	'setSf2PreviewAttendance',
	'exportSf2Workbook',
	'openSf2Workbook',
	'presentAllSf2PreviewAttendance',
	'syncAndOpenSf2Workbook',
	'killAllExcelProcesses',
	// sf2 months
	'getSf2MonthPreview',
	'getSf2LaunchMonth',
	'createSf2MonthFile',
	'listSf2MonthWorkbooks',
	'getSf2SchoolCalendarSettings',
	'setSf2SchoolStartDate',
	'runSf2WorkbookSplit',
	// The Students page's roster sync: the *Sync Roster* buttons it replaced are
	// gone, so this is the only way a roster reaches the month worksheets.
	'refreshSf2MonthRoster'
];

/** Deliberately not ported. Each one is a call site to delete, not a gap to fill. */
const RETIRED = [
	// D15: the startup self-heal
	'healCurrentMonthWorkbook',
	'onSf2HealOutcome',
	'SF2_HEAL_OUTCOME_EVENT',
	// D10: Google Drive and the sync folder
	'chooseBackupSyncFolder',
	'clearBackupSyncFolder',
	'connectGoogleDriveBackup',
	'disconnectGoogleDriveBackup',
	'uploadLatestBackupToGoogleDrive',
	'chooseRestoreDatabaseFile'
];

const surface = api as unknown as Record<string, unknown>;

describe('$lib/api', () => {
	it.each(SURVIVED)('still exports %s', (name) => {
		expect(surface[name]).toBeTypeOf('function');
	});

	it.each(RETIRED)('no longer exports %s', (name) => {
		expect(surface[name]).toBeUndefined();
	});
});
