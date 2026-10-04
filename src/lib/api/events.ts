/**
 * `$lib/api/events.ts` → `$lib/db/repos/events`.
 *
 * Every name, parameter and return shape already matched, so this is a re-export.
 * `listAttendanceAudit`'s filter object keeps its Rust meaning too: `eventId` and
 * `studentId` are alternatives, not a conjunction.
 */

export {
	addEvent,
	addEvents,
	deleteEvent,
	deleteEvents,
	lastEventForStudent,
	listAttendanceAudit,
	listEvents,
	listEventsForDate,
	listEventsForStudent,
	updateEvent
} from '$lib/db/repos/events';

export type {
	AttendanceAuditEntry,
	AttendanceEvent,
	CreateEventRequest,
	UpdateEventRequest
} from '$lib/types';
