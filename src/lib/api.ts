/// API client for the attendance system
import type {
	Student,
	AttendanceEvent,
	Settings,
	CreateStudentRequest,
	UpdateStudentRequest,
	CreateEventRequest,
	ExportData
} from './types';

// Get API base URL from environment or default to localhost
const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3030';

class ApiClient {
	private baseUrl: string;

	constructor(baseUrl: string = API_BASE_URL) {
		this.baseUrl = baseUrl;
	}

	private async request<T>(endpoint: string, options?: RequestInit): Promise<T> {
		const url = `${this.baseUrl}${endpoint}`;
		const response = await fetch(url, {
			...options,
			headers: {
				'Content-Type': 'application/json',
				...options?.headers
			}
		});

		if (!response.ok) {
			const error = await response.json().catch(() => ({ error: 'Unknown error' }));
			throw new Error(error.error || `HTTP ${response.status}`);
		}

		// Handle 204 No Content
		if (response.status === 204) {
			return undefined as T;
		}

		return response.json();
	}

	// ============================================================================
	// Students
	// ============================================================================

	async listStudents(): Promise<Student[]> {
		return this.request<Student[]>('/api/students');
	}

	async getStudent(id: string): Promise<Student> {
		return this.request<Student>(`/api/students/${id}`);
	}

	async findStudentByCard(serial: string): Promise<Student | undefined> {
		try {
			return await this.request<Student>(`/api/students/card/${encodeURIComponent(serial)}`);
		} catch {
			// 404 means not found
			return undefined;
		}
	}

	async createStudent(req: CreateStudentRequest): Promise<Student> {
		return this.request<Student>('/api/students', {
			method: 'POST',
			body: JSON.stringify(req)
		});
	}

	async updateStudent(id: string, req: UpdateStudentRequest): Promise<Student> {
		return this.request<Student>(`/api/students/${id}`, {
			method: 'PUT',
			body: JSON.stringify(req)
		});
	}

	async deleteStudent(id: string): Promise<void> {
		return this.request<void>(`/api/students/${id}`, {
			method: 'DELETE'
		});
	}

	// ============================================================================
	// Events
	// ============================================================================

	async listEvents(): Promise<AttendanceEvent[]> {
		return this.request<AttendanceEvent[]>('/api/events');
	}

	async listEventsForStudent(studentId: string): Promise<AttendanceEvent[]> {
		return this.request<AttendanceEvent[]>(`/api/events/student/${studentId}`);
	}

	async lastEventForStudent(studentId: string): Promise<AttendanceEvent | undefined> {
		try {
			return await this.request<AttendanceEvent>(`/api/events/student/${studentId}/last`);
		} catch {
			return undefined;
		}
	}

	async createEvent(req: CreateEventRequest): Promise<AttendanceEvent> {
		return this.request<AttendanceEvent>('/api/events', {
			method: 'POST',
			body: JSON.stringify(req)
		});
	}

	async deleteEvent(id: string): Promise<void> {
		return this.request<void>(`/api/events/${id}`, {
			method: 'DELETE'
		});
	}

	// ============================================================================
	// Settings
	// ============================================================================

	async getSettings(): Promise<Settings> {
		return this.request<Settings>('/api/settings');
	}

	async updateSettings(settings: Settings): Promise<Settings> {
		return this.request<Settings>('/api/settings', {
			method: 'PUT',
			body: JSON.stringify(settings)
		});
	}

	// ============================================================================
	// Data Management
	// ============================================================================

	async exportAll(): Promise<ExportData> {
		return this.request<ExportData>('/api/export');
	}

	async importAll(data: ExportData): Promise<void> {
		return this.request<void>('/api/import', {
			method: 'POST',
			body: JSON.stringify(data)
		});
	}

	async wipeAll(): Promise<void> {
		return this.request<void>('/api/wipe', {
			method: 'POST'
		});
	}

	// ============================================================================
	// Health Check
	// ============================================================================

	async healthCheck(): Promise<{ status: string; timestamp: string }> {
		return this.request('/api/health');
	}
}

// Export singleton instance
export const api = new ApiClient();

// Export types
export type {
	Student,
	AttendanceEvent,
	Settings,
	CreateStudentRequest,
	UpdateStudentRequest,
	CreateEventRequest,
	ExportData
};
