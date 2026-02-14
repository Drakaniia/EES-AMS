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
    lrn?: string;
    last_name: string;
    first_name: string;
    middle_name?: string;
    gender?: string;
    birthday?: string;
    age?: number;
    mother_name?: string;
    father_name?: string;
    guardian_name?: string;
    address?: string;
    class_id?: number;
    created_at?: string;
    updated_at?: string;
}

export interface ImportResult {
    success_count: number;
    error_count: number;
    errors: string[];
    imported_students: Student[];
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

export interface UpdateStatus {
    available: boolean;
    current_version: string;
    latest_version?: string;
    body?: string;
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
                // Open auth URL using Tauri shell command
                try {
                    await invoke('open_url', { url: response.data });
                } catch (error) {
                    console.error('Failed to open browser:', error);
                    // Fallback to window.open
                    window.open(response.data, '_blank');
                }
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

        syncWithData: async (data: unknown): Promise<ApiResponse<boolean>> => {
            return await invoke<ApiResponse<boolean>>('google_sync_data', { data });
        },
    },

    fs: {
        writeFile: async (path: string, contents: Uint8Array): Promise<void> => {
            return await invoke('fs_write_file', { path, contents });
        },

        removeFile: async (path: string): Promise<void> => {
            return await invoke('fs_remove_file', { path });
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

        importFromExcel: async (filePath: string, classId?: number): Promise<ImportResult> => {
            return await invoke<ImportResult>('student_import_from_excel', { filePath, classId });
        },
    },

    updater: {
        checkForUpdates: async (): Promise<ApiResponse<UpdateStatus>> => {
            return await invoke<ApiResponse<UpdateStatus>>('check_for_updates');
        },

        downloadAndInstall: async (): Promise<ApiResponse<string>> => {
            return await invoke<ApiResponse<string>>('download_and_install_update');
        },

        restart: async (): Promise<ApiResponse> => {
            return await invoke<ApiResponse>('restart_app');
        },

        onUpdateProgress: (callback: (progress: string) => void) => {
            import('@tauri-apps/api/event').then(({ listen }) => {
                listen('update-progress', (event) => {
                    callback(event.payload as string);
                });
            });
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
