import { invoke } from '@tauri-apps/api/core';
import type { Student, StudentGender, CreateStudentRequest } from '../types';
export type { Student, StudentGender, CreateStudentRequest } from '../types';

export async function listStudents(classId?: string): Promise<Student[]> {
	return await invoke('list_students', { classId });
}

export async function getStudent(id: string): Promise<Student> {
	return await invoke('get_student', { id });
}

export async function findStudentByCard(serial: string): Promise<Student | undefined> {
	return await invoke('find_student_by_card', { serial });
}

export async function saveStudent(student: Student): Promise<Student> {
	if (student.id) {
		return await invoke('update_student', {
			id: student.id,
			req: {
				name: student.name,
				gender: student.gender,
				cardSerial: student.cardSerial,
				classId: student.classId
			}
		});
	} else {
		return await invoke('create_student', {
			req: {
				name: student.name,
				gender: student.gender,
				cardSerial: student.cardSerial,
				classId: student.classId
			}
		});
	}
}

export async function createStudents(students: CreateStudentRequest[]): Promise<Student[]> {
	return await invoke('create_students', { reqs: students });
}

export async function deleteStudent(id: string): Promise<void> {
	return await invoke('delete_student', { id });
}

export const uid = () => crypto.randomUUID();
