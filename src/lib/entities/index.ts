// Domain entity types

export interface Teacher {
	id: string;
	employeeId: string;
	firstName: string;
	lastName: string;
	department: string;
	position: string;
	sfcCardId?: string;
	createdAt: string;
	updatedAt: string;
}

export interface AttendanceRecord {
	id: string;
	teacherId: string;
	date: string;
	timeIn?: string;
	timeOut?: string;
	status: 'present' | 'absent' | 'late' | 'half-day';
	sfcCardId?: string;
	createdAt: string;
}

export interface SfcCard {
	id: string;
	cardNumber: string;
	teacherId?: string;
	isActive: boolean;
	registeredAt: string;
}
