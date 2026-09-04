import { goto } from '$app/navigation';
import { listStudents, type Student } from '$lib/db-rust';
import type { PaletteGroup, PaletteItem, PaletteGroupResult } from '$lib/command-palette';

export type { PaletteGroup, PaletteItem, PaletteGroupResult };

/**
 * Global command palette (Ctrl/Cmd+K).
 *
 * - Static **Pages** cover every destination, including Settings sections
 *   (deep-linked via `#section-id`).
 * - Routes register contextual **Actions** with `register()` while mounted and
 *   remove them with `unregister()` on teardown.
 * - **Students** are fuzzy-matched by name and jump to their attendance log.
 *
 * Card-reader safety: while the attendance page has its card wedge input
 * armed (`setCardReaderArmed(true)`), the palette refuses to open so raw
 * card scans never leak into the query box.
 */

const PAGE_ITEMS: PaletteItem[] = [
	{
		id: 'page-reports',
		label: 'SF2 Reports',
		keywords: 'sf2 excel deped report monthly matrix',
		hint: '/reports',
		group: 'Pages',
		run: () => void goto('/reports')
	},
	{
		id: 'page-attendance',
		label: 'Take Attendance',
		keywords: 'live session card reader check in scan tap',
		hint: '/attendance',
		group: 'Pages',
		run: () => void goto('/attendance')
	},
	{
		id: 'page-overview',
		label: 'Daily Overview',
		keywords: 'today summary dashboard counts',
		hint: '/attendance/overview',
		group: 'Pages',
		run: () => void goto('/attendance/overview')
	},
	{
		id: 'page-logs',
		label: 'Attendance Logs',
		keywords: 'records audit history entries',
		hint: '/attendance/logs',
		group: 'Pages',
		run: () => void goto('/attendance/logs')
	},
	{
		id: 'page-students',
		label: 'Class List',
		keywords: 'students roster class list',
		hint: '/students',
		group: 'Pages',
		run: () => void goto('/students')
	},
	{
		id: 'page-settings',
		label: 'Settings',
		keywords: 'configuration preferences options',
		hint: '/settings',
		group: 'Pages',
		run: () => void goto('/settings')
	},
	{
		id: 'settings-classes',
		label: 'Settings · Classes & Schedule',
		keywords: 'class room sessions schedule',
		hint: '/settings',
		group: 'Pages',
		run: () => void goto('/settings#settings-classes')
	},
	{
		id: 'settings-sf2',
		label: 'Settings · SF2 Workbook',
		keywords: 'sf2 template excel import',
		hint: '/settings',
		group: 'Pages',
		run: () => void goto('/settings#settings-sf2')
	},
	{
		id: 'settings-backup',
		label: 'Settings · Data Management',
		keywords: 'backup restore data google drive',
		hint: '/settings',
		group: 'Pages',
		run: () => void goto('/settings#settings-backup')
	},
	{
		id: 'settings-branding',
		label: 'Settings · App Branding',
		keywords: 'logo title appearance school name',
		hint: '/settings',
		group: 'Pages',
		run: () => void goto('/settings#settings-branding')
	},
	{
		id: 'settings-global',
		label: 'Settings · Global Settings',
		keywords: 'defaults day start end late time zone',
		hint: '/settings',
		group: 'Pages',
		run: () => void goto('/settings#settings-global')
	},
	{
		id: 'settings-update',
		label: 'Settings · Software Update',
		keywords: 'update version upgrade',
		hint: '/settings',
		group: 'Pages',
		run: () => void goto('/settings#settings-update')
	}
];

class CommandPaletteStore {
	open = $state(false);
	query = $state('');
	selectedIndex = $state(0);
	/** True while the attendance card-reader input is armed (wedge safety). */
	cardReaderArmed = $state(false);

	/** Contextual actions contributed by currently-mounted routes. */
	registered = $state<Record<string, PaletteItem>>({});

	students = $state<Student[]>([]);
	studentsLoading = $state(false);
	studentsFailed = $state(false);
	private studentsLoaded = false;

	openPalette() {
		if (this.open || this.cardReaderArmed) return;
		this.query = '';
		this.selectedIndex = 0;
		this.open = true;
		void this.ensureStudents();
	}

	closePalette() {
		this.open = false;
		this.query = '';
		this.selectedIndex = 0;
	}

	register(item: PaletteItem) {
		this.registered[item.id] = item;
	}

	unregister(id: string) {
		delete this.registered[id];
	}

	setCardReaderArmed(armed: boolean) {
		this.cardReaderArmed = armed;
	}

	run(item: PaletteItem) {
		this.closePalette();
		item.run();
	}

	/** All candidate items: static pages + route actions + students. */
	allItems(): PaletteItem[] {
		const registered = Object.values(this.registered);
		const studentItems: PaletteItem[] = this.students.map((student) => ({
			id: `student-${student.id}`,
			label: student.name,
			keywords: 'student attendance log records',
			hint: 'Attendance log',
			group: 'Students',
			run: () => void goto(`/students?student=${encodeURIComponent(student.id)}`)
		}));
		return [...PAGE_ITEMS, ...registered, ...studentItems];
	}

	private async ensureStudents() {
		if (this.studentsLoaded || this.studentsLoading) return;
		this.studentsLoading = true;
		try {
			this.students = await listStudents();
			this.studentsLoaded = true;
		} catch {
			this.studentsFailed = true;
		} finally {
			this.studentsLoading = false;
		}
	}
}

export const commandPaletteStore = new CommandPaletteStore();
