/// Type definitions matching the Rust backend

export type StudentGender = 'male' | 'female';

export interface Student {
	id: string;
	name: string;
	gender?: StudentGender;
	cardSerial?: string;
	classId?: string;
	createdAt: string;
}

export interface Session {
	name: string;
	startTime: string;
	endTime: string;
	lateAfter: string;
}

export interface Class {
	id: string;
	name: string;
	room?: string;
	dayStart: string;
	dayEnd: string;
	lateAfter: string;
	sessions: Session[];
	days: number[];
	createdAt: string;
}

export type AttendanceType = 'in';
export type AttendanceMode = 'manual' | 'card_reader';

export interface AttendanceEvent {
	id: string;
	studentId: string;
	classId?: string;
	type: AttendanceType;
	timestamp: string;
	note?: string;
	sessionKey?: string;
	overrideReason?: string;
	updatedAt?: string;
}

export interface AttendanceAuditEntry {
	id: string;
	eventId?: string;
	studentId: string;
	classId?: string;
	sessionKey?: string;
	action: 'create_override' | 'update' | 'delete';
	reason: string;
	beforeJson?: string;
	afterJson?: string;
	createdAt: string;
	actor: string;
}

export interface AuditEvent {
	id: string;
	entityType: string;
	entityId?: string;
	action: string;
	summary: string;
	beforeJson?: string;
	afterJson?: string;
	metadataJson?: string;
	createdAt: string;
	actor: string;
}

export interface Settings {
	id: string;
	dayStart: string;
	dayEnd: string;
	lateAfter: string;
	quarter: string;
	attendanceMode: AttendanceMode;
	q1Start?: string;
	q1End?: string;
	q2Start?: string;
	q2End?: string;
	q3Start?: string;
	q3End?: string;
}

export interface CreateStudentRequest {
	name: string;
	gender?: StudentGender;
	cardSerial?: string;
	classId?: string;
}

export interface UpdateStudentRequest {
	name?: string;
	gender?: StudentGender;
	cardSerial?: string;
	classId?: string;
}

export interface CreateClassRequest {
	name: string;
	room?: string;
	dayStart: string;
	dayEnd: string;
	lateAfter: string;
	sessions: Session[];
	days: number[];
}

export interface UpdateClassRequest {
	name?: string;
	room?: string;
	dayStart?: string;
	dayEnd?: string;
	lateAfter?: string;
	sessions?: Session[];
	days?: number[];
}

export interface CreateEventRequest {
	studentId: string;
	classId?: string;
	type: AttendanceType;
	note?: string;
	sessionKey?: string;
	overrideReason?: string;
	timestamp?: string;
}

export interface UpdateEventRequest {
	classId?: string;
	timestamp?: string;
	note?: string;
	sessionKey?: string;
	reason: string;
}

export interface ExportData {
	students: Student[];
	classes: Class[];
	events: AttendanceEvent[];
	settings: Settings[];
	auditEvents?: AuditEvent[];
	exportedAt: number;
}

export type BackupKind = 'auto' | 'manual' | 'pre_restore' | 'unknown';

export interface BackupSummary {
	path: string;
	fileName: string;
	createdAt: number;
	sizeBytes: number;
	kind: BackupKind;
}

export interface BackupStatus {
	localBackupDir: string;
	backupCount: number;
	retentionLimit: number;
	lastBackupAt?: number;
	lastBackupPath?: string;
	syncFolderPath?: string;
	lastError?: string;
	lastSyncError?: string;
	googleDriveConfigured: boolean;
	googleDriveConnected: boolean;
	googleDriveFolderId?: string;
	googleDriveFolderName?: string;
	lastGoogleDriveBackupAt?: number;
	lastGoogleDriveFileId?: string;
	lastGoogleDriveError?: string;
}

export interface BackupPreview {
	sourcePath: string;
	fileName: string;
	modifiedAt: number;
	sizeBytes: number;
	schemaVersion: number;
	studentCount: number;
	classCount: number;
	eventCount: number;
	settingsCount: number;
	sf2TemplateCount: number;
	warnings: string[];
}

export interface RestoreResult {
	restoredPath: string;
	preRestoreBackupPath: string;
	restoredAt: number;
	schemaVersion: number;
	migrated: boolean;
	warnings: string[];
}

export interface ServerInfo {
	localIp: string;
	port: number;
	url: string;
}

export interface Sf2ImportSummary {
	templateId: string;
	classId: string;
	className: string;
	sourcePath: string;
	schoolYear: string;
	gradeLevel: string;
	section: string;
	learnersFound: number;
	studentsCreated: number;
	studentsReused: number;
	datesMapped: number;
}

export interface Sf2ImportValidation {
	sourcePath: string;
	classId?: string;
	className: string;
	currentStudentCount: number;
	sf2LearnerCount: number;
	missingFromSf2: Sf2ValidationStudent[];
	missingFromCurrent: Sf2ValidationLearner[];
	possibleNameMismatches: Sf2ValidationNameMismatch[];
	duplicateCurrentStudents: Sf2ValidationDuplicate[];
	duplicateSf2Learners: Sf2ValidationDuplicate[];
	missingLearnerInfo: Sf2ValidationLearner[];
	hasDiscrepancies: boolean;
}

export interface Sf2ValidationStudent {
	studentId: string;
	name: string;
	normalizedName: string;
	gender?: string;
}

export interface Sf2ValidationLearner {
	rowIndex: number;
	name: string;
	normalizedName: string;
	genderBlock?: string;
}

export interface Sf2ValidationNameMismatch {
	currentStudent: Sf2ValidationStudent;
	sf2Learner: Sf2ValidationLearner;
	reason: string;
}

export interface Sf2ValidationDuplicate {
	normalizedName: string;
	names: string[];
	studentIds: string[];
	rowIndexes: number[];
}

export interface Sf2TemplateDraft {
	classId?: string;
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	firstSchoolDay?: number;
	learnerNames: string[];
}

export interface Sf2WorkbookSettings {
	templateId: string;
	classId: string;
	className: string;
	sourcePath: string;
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	firstSchoolDay: number;
	learnerNames: string[];
	datesMapped: number;
}

export interface Sf2CloseDaySummary {
	classId: string;
	date: string;
	presentCount: number;
	absentCount: number;
}

export interface Sf2TemplateSummary {
	id: string;
	sourcePath: string;
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	classId: string;
	importedAt: number;
}

export interface Sf2ExportReadiness {
	template?: Sf2TemplateSummary;
	closedDays: string[];
	mappedStudents: number;
	mappedDates: number;
	canExport: boolean;
	issues: string[];
	warnings: string[];
}

export type Sf2PreviewCellStatus = 'present' | 'absent' | 'open';

export interface Sf2ExportPreview {
	template?: Sf2TemplateSummary;
	classId?: string;
	className: string;
	sourcePath?: string;
	dates: Sf2PreviewDate[];
	students: Sf2PreviewStudentRow[];
	absentList: Sf2PreviewAbsence[];
	closedDays: string[];
	mappedStudents: number;
	mappedDates: number;
	presentCount: number;
	absenceCount: number;
	unmappedStudentCount: number;
	unmappedClosedDayCount: number;
	canExport: boolean;
	issues: string[];
	warnings: string[];
}

export interface Sf2PreviewDate {
	date: string;
	sheetName: string;
	columnLetter: string;
	columnIndex: number;
	closed: boolean;
}

export interface Sf2PreviewStudentRow {
	studentId: string;
	studentName: string;
	workbookName: string;
	gender?: string;
	rowIndex: number;
	mapped: boolean;
	presentCount: number;
	absentCount: number;
	warnings: string[];
	cells: Sf2PreviewCell[];
}

export interface Sf2PreviewCell {
	date: string;
	status: Sf2PreviewCellStatus;
	editable: boolean;
}

export interface Sf2PreviewAbsence {
	studentId: string;
	studentName: string;
	date: string;
	rowIndex: number;
}

export interface Sf2ExportResult {
	outputPath: string;
	marksWritten: number;
	closedDays: number;
}
