import { saveClass, deleteClass, type Class, type Session } from '$lib/features/settings/native';
import type { Ctx } from './state-context';

/**
 * Class & section management state and actions.
 * Owns all form state for the class dialog and delete confirmation.
 *
 * Singleton pattern: imported by both orchestrator and components.
 * The orchestrator calls `.init(ctx)` to wire cross-cutting services.
 */
class ClassState {
	ctx!: Ctx;

	init(ctx: Ctx) {
		this.ctx = ctx;
	}

	// ── State ──────────────────────────────────────────────────────────────────
	classes = $state<Class[]>([]);
	classDialogOpen = $state(false);
	editingClass = $state<Class | null>(null);
	formClassName = $state('');
	formRoom = $state('');
	formDayStart = $state('');
	formDayEnd = $state('');
	formLateAfter = $state('');
	formSessions = $state<Session[]>([]);
	formDays = $state<number[]>([1, 2, 3, 4, 5]);
	sessionMode = $state<'single' | 'morning-afternoon' | 'custom'>('single');
	deleteTarget = $state<{ id: string; name: string } | null>(null);

	// ── Actions ─────────────────────────────────────────────────────────────────
	openAddClass(defaultDayStart: string, defaultDayEnd: string, defaultLateAfter: string) {
		if (this.classes.length > 0) {
			this.ctx.toast('One class is already configured. Edit the existing class instead.', false);
			return;
		}
		this.editingClass = null;
		this.formClassName = '';
		this.formRoom = '';
		this.formDayStart = defaultDayStart;
		this.formDayEnd = defaultDayEnd;
		this.formLateAfter = defaultLateAfter;
		this.formSessions = [
			{
				name: 'Full Day',
				startTime: defaultDayStart,
				endTime: defaultDayEnd,
				lateAfter: defaultLateAfter
			}
		];
		this.formDays = [1, 2, 3, 4, 5];
		this.sessionMode = 'single';
		this.classDialogOpen = true;
	}

	openEditClass(c: Class) {
		this.editingClass = c;
		this.formClassName = c.name;
		this.formRoom = c.room ?? '';
		this.formDayStart = c.dayStart;
		this.formDayEnd = c.dayEnd;
		this.formLateAfter = c.lateAfter;
		this.formSessions =
			c.sessions && c.sessions.length > 0
				? JSON.parse(JSON.stringify(c.sessions))
				: [
						{
							name: 'Full Day',
							startTime: c.dayStart,
							endTime: c.dayEnd,
							lateAfter: c.lateAfter
						}
					];
		this.formDays = c.days && c.days.length > 0 ? [...c.days] : [1, 2, 3, 4, 5];

		if (this.formSessions.length === 1 && this.formSessions[0].name === 'Full Day') {
			this.sessionMode = 'single';
		} else if (
			this.formSessions.length === 2 &&
			this.formSessions[0].name === 'Morning' &&
			this.formSessions[1].name === 'Afternoon'
		) {
			this.sessionMode = 'morning-afternoon';
		} else {
			this.sessionMode = 'custom';
		}

		this.classDialogOpen = true;
	}

	async onSaveClass(e: SubmitEvent) {
		e.preventDefault();
		const name = this.formClassName.trim();
		if (!name) return;

		const primary = this.formSessions[0] || {
			startTime: this.formDayStart,
			endTime: this.formDayEnd,
			lateAfter: this.formLateAfter
		};

		const c: Class = {
			id: this.editingClass?.id ?? '',
			name,
			room: this.formRoom.trim(),
			dayStart: primary.startTime,
			dayEnd: primary.endTime,
			lateAfter: primary.lateAfter,
			sessions: this.formSessions,
			days: this.formDays,
			createdAt: this.editingClass?.createdAt ?? ''
		};

		try {
			await saveClass(c, !!this.editingClass);
			this.ctx.toast(this.editingClass ? 'Class updated' : 'Class added');
			this.classDialogOpen = false;
			await this.ctx.reload();
		} catch (error) {
			this.ctx.toast(`Failed to save class: ${error}`, false);
		}
	}

	async confirmDeleteClass(target = this.deleteTarget) {
		if (!target) return;
		await deleteClass(target.id);
		this.ctx.toast('Class deleted');
		this.deleteTarget = null;
		await this.ctx.reload();
	}

	async onDeleteClass(event: MouseEvent, id: string) {
		const classToDelete = this.classes.find((c) => c.id === id);
		if (!classToDelete) return;

		const target = { id: classToDelete.id, name: classToDelete.name };
		if (event.shiftKey) {
			await this.confirmDeleteClass(target);
			return;
		}

		this.deleteTarget = target;
	}
}

export const classState = new ClassState();
