// Tauri API Bridge - Replaces Electron IPC with Tauri commands

import { invoke } from '@tauri-apps/api/core';

// Type definitions
export interface Class {
    id?: number;
    name: string;
    section?: string;
    school_year?: string;
    created_at?: string;
    updated_at?: string;
}

export interface Student {
    id?: number;
    student_id: string;
    first_name: string;
    last_name: string;
    class_id?: number;
    created_at?: string;
    updated_at?: string;
}

export interface AttendanceRecord {
    id?: number;
    student_id: number;
    class_id: number;
    date: string;
    status: 'present' | 'absent' | 'late' | 'excused';
    notes?: string;
    synced?: number;
    created_at?: string;
}

export interface GoogleCredentials {
    clientId: string;
    clientSecret: string;
    redirectUri: string;
}

export interface SyncStatus {
    is_online: boolean;
    last_sync_time: string | null;
    pending_records: number;
    is_syncing: boolean;
    error: string | null;
}

export interface AttendanceStats {
    total_students: number;
    present_today: number;
    absent_today: number;
    late_today: number;
    attendance_rate: number;
}

export interface ApiResponse<T = unknown> {
    success: boolean;
    data?: T;
    id?: number;
    error?: string;
}

// Tauri API - mimics the Electron API structure
export const tauriAPI = {
    class: {
        create: async (data: Class): Promise<ApiResponse<{ id: number }>> => {
            const response = await invoke<ApiResponse<number>>('class_create', { 
                input: {
                    name: data.name,
                    section: data.section,
                    school_year: data.school_year
                }
            });
            
            if (response.success && response.id !== undefined) {
                return { success: true, data: { id: response.id } };
            }
            return { success: false, error: response.error };
        },

        getAll: async (): Promise<ApiResponse<Class[]>> => {
            return await invoke<ApiResponse<Class[]>>('class_get_all');
        },

        delete: async (id: number): Promise<ApiResponse> => {
            return await invoke<ApiResponse>('class_delete', { id });
        },
    },

    student: {
        create: async (data: Student): Promise<ApiResponse<{ id: number }>> => {
            const response = await invoke<ApiResponse<number>>('student_create', {
                input: {
                    student_id: data.student_id,
                    first_name: data.first_name,
                    last_name: data.last_name,
                    class_id: data.class_id
                }
            });
            
            if (response.success && response.id !== undefined) {
                return { success: true, data: { id: response.id } };
            }
            return { success: false, error: response.error };
        },

        getByClass: async (classId: number): Promise<ApiResponse<Student[]>> => {
            return await invoke<ApiResponse<Student[]>>('student_get_by_class', { classId });
        },

        getAll: async (): Promise<ApiResponse<Student[]>> => {
            return await invoke<ApiResponse<Student[]>>('student_get_all');
        },

        delete: async (id: number): Promise<ApiResponse> => {
            return await invoke<ApiResponse>('student_delete', { id });
        },
    },

    attendance: {
        record: async (data: AttendanceRecord): Promise<ApiResponse<{ id: number }>> => {
            const response = await invoke<ApiResponse<number>>('attendance_record', {
                input: {
                    student_id: data.student_id,
                    class_id: data.class_id,
                    date: data.date,
                    status: data.status,
                    notes: data.notes
                }
            });
            
            if (response.success && response.id !== undefined) {
                return { success: true, data: { id: response.id } };
            }
            return { success: false, error: response.error };
        },

        getByClassAndDate: async (classId: number, date: string): Promise<ApiResponse<AttendanceRecord[]>> => {
            return await invoke<ApiResponse<AttendanceRecord[]>>('attendance_get_by_class_and_date', { classId, date });
        },

        getUnsynced: async (): Promise<ApiResponse<AttendanceRecord[]>> => {
            return await invoke<ApiResponse<AttendanceRecord[]>>('attendance_get_unsynced');
        },

        getStats: async (classId: number): Promise<ApiResponse<AttendanceStats>> => {
            return await invoke<ApiResponse<AttendanceStats>>('attendance_get_stats', { classId });
        },
    },

    google: {
        saveCredentials: async (credentials: GoogleCredentials): Promise<ApiResponse> => {
            return await invoke<ApiResponse>('google_save_credentials', { credentials });
        },

        isAuthenticated: async (): Promise<ApiResponse<boolean>> => {
            return await invoke<ApiResponse<boolean>>('google_is_authenticated');
        },

        startAuth: async (): Promise<ApiResponse<string>> => {
            const response = await invoke<ApiResponse<string>>('google_start_auth');
            if (response.success && response.data) {
                // Open auth URL in default browser
                window.open(response.data, '_blank');
            }
            return response;
        },

        handleCallback: async (code: string): Promise<ApiResponse<boolean>> => {
            return await invoke<ApiResponse<boolean>>('google_handle_callback', { code });
        },

        logout: async (): Promise<ApiResponse> => {
            return await invoke<ApiResponse>('google_logout');
        },

        sync: async (): Promise<ApiResponse<boolean>> => {
            return await invoke<ApiResponse<boolean>>('google_sync');
        },

        getSyncStatus: async (): Promise<ApiResponse<SyncStatus>> => {
            return await invoke<ApiResponse<SyncStatus>>('google_get_sync_status');
        },
    },
};

// Make it available globally for compatibility with existing code
declare global {
    interface Window {
        electronAPI: typeof tauriAPI;
    }
}

// Assign to window for backward compatibility
if (typeof window !== 'undefined') {
    window.electronAPI = tauriAPI;
}

export default tauriAPI;
