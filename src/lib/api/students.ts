/**
 * `$lib/api/students.ts` → `$lib/db/repos/students`.
 *
 * The repo was written against this exact call surface, so the adapter is a
 * re-export and nothing else: no shape mapping, no defaulting, no second opinion
 * about what a student is. Anything added here would be a behaviour the UI never
 * had.
 */

export {
	createStudents,
	deleteStudent,
	findStudentByCard,
	getStudent,
	listStudents,
	saveStudent
} from '$lib/db/repos/students';

export type { Student, StudentGender, CreateStudentRequest } from '$lib/types';

/** Client-generated row id for an unsaved student. `crypto` is a browser global. */
export const uid = (): string => crypto.randomUUID();
