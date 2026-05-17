/// Type definitions matching the Rust backend

export interface Student {
	id: string;
	name: string;
	studentNumber: string;
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

export type AttendanceType = 'in' | 'out';

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
	q1Start?: string;
	q1End?: string;
	q2Start?: string;
	q2End?: string;
	q3Start?: string;
	q3End?: string;
	q4Start?: string;
	q4End?: string;
}

export interface CreateStudentRequest {
	name: string;
	studentNumber: string;
	cardSerial?: string;
	classId?: string;
}

export interface UpdateStudentRequest {
	name?: string;
	studentNumber?: string;
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
