/// Type definitions matching the Rust backend

export interface Student {
	id: string;
	name: string;
	studentNumber: string;
	cardSerial?: string;
	createdAt: string;
}

export type AttendanceType = 'in' | 'out';

export interface AttendanceEvent {
	id: string;
	studentId: string;
	type: AttendanceType;
	timestamp: string;
	note?: string;
}

export interface Settings {
	className: string;
	dayStart: string;
	dayEnd: string;
	lateAfter: string;
}

export interface CreateStudentRequest {
	name: string;
	studentNumber: string;
	cardSerial?: string;
}

export interface UpdateStudentRequest {
	name?: string;
	studentNumber?: string;
	cardSerial?: string;
}

export interface CreateEventRequest {
	studentId: string;
	type: AttendanceType;
	note?: string;
}

export interface ExportData {
	students: Student[];
	events: AttendanceEvent[];
	settings: Settings;
	exportedAt: string;
}

export interface ServerInfo {
	localIp: string;
	port: number;
	url: string;
}
