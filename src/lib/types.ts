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
}

export interface ExportData {
	students: Student[];
	classes: Class[];
	events: AttendanceEvent[];
	settings: Settings[];
	exportedAt: number;
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

export interface Sf2ExportResult {
	outputPath: string;
	marksWritten: number;
	closedDays: number;
}
