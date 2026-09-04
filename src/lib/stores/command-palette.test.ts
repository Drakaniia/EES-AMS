import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { commandPaletteStore as StoreValue } from '$lib/stores/command-palette.svelte';
import type { PaletteItem } from '$lib/command-palette';

type Store = typeof StoreValue;

const mocks = vi.hoisted(() => ({
	goto: vi.fn(),
	listStudents: vi.fn()
}));

vi.mock('$app/navigation', () => ({ goto: mocks.goto }));
vi.mock('$lib/db-rust', () => ({ listStudents: mocks.listStudents }));

async function freshStore(): Promise<Store> {
	// Test-only: module reload gives each test a pristine singleton,
	// intentionally exercising the module-loading boundary.
	vi.resetModules();
	const mod = await import('$lib/stores/command-palette.svelte');
	return mod.commandPaletteStore;
}

const students = [
	{ id: 's1', name: 'Juan Dela Cruz', createdAt: '2026-06-01' },
	{ id: 's2', name: 'Maria Santos', createdAt: '2026-06-01' },
	{ id: 's3', name: 'Jose Ramirez', createdAt: '2026-06-01' }
];

describe('command palette store', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mocks.listStudents.mockResolvedValue(students);
	});

	it('opens the palette, resets query, and lazy-loads students', async () => {
		const store = await freshStore();
		store.query = 'leftover';
		store.selectedIndex = 3;

		store.openPalette();

		expect(store.open).toBe(true);
		expect(store.query).toBe('');
		expect(store.selectedIndex).toBe(0);
		expect(mocks.listStudents).toHaveBeenCalledTimes(1);
		await vi.waitFor(() => expect(store.students).toHaveLength(3));
	});

	it('loads students only once across multiple opens', async () => {
		const store = await freshStore();
		store.openPalette();
		await vi.waitFor(() => expect(store.students).toHaveLength(3));

		store.closePalette();
		store.openPalette();

		expect(mocks.listStudents).toHaveBeenCalledTimes(1);
	});

	it('does not open while the card-reader input is armed', async () => {
		const store = await freshStore();
		store.setCardReaderArmed(true);
		store.openPalette();

		expect(store.open).toBe(false);
		expect(mocks.listStudents).not.toHaveBeenCalled();

		store.setCardReaderArmed(false);
		store.openPalette();
		expect(store.open).toBe(true);
	});

	it('registers and unregisters contextual actions', async () => {
		const store = await freshStore();
		const item: PaletteItem = {
			id: 'month-switch',
			label: 'Switch Month',
			keywords: '',
			hint: '',
			group: 'Actions',
			run: vi.fn()
		};

		store.register(item);
		expect(store.allItems().some((i) => i.id === 'month-switch')).toBe(true);

		store.unregister('month-switch');
		expect(store.allItems().some((i) => i.id === 'month-switch')).toBe(false);
	});

	it('always exposes the static page items', async () => {
		const store = await freshStore();
		const ids = store.allItems().map((i) => i.id);
		for (const id of [
			'page-reports',
			'page-attendance',
			'page-overview',
			'page-logs',
			'page-students',
			'page-settings',
			'settings-classes',
			'settings-sf2',
			'settings-backup',
			'settings-branding',
			'settings-global',
			'settings-update'
		]) {
			expect(ids).toContain(id);
		}
	});

	it('runs an item and closes the palette', async () => {
		const store = await freshStore();
		store.openPalette();
		const run = vi.fn();
		store.register({
			id: 'test-action',
			label: 'Test Action',
			keywords: '',
			hint: '',
			group: 'Actions',
			run
		});

		const item = store.allItems().find((i) => i.id === 'test-action')!;
		store.run(item);

		expect(run).toHaveBeenCalledTimes(1);
		expect(store.open).toBe(false);
	});

	it('navigates to reports when a page item runs', async () => {
		const store = await freshStore();
		const item = store.allItems().find((i) => i.id === 'page-reports')!;
		store.run(item);
		expect(mocks.goto).toHaveBeenCalledWith('/reports');
	});

	it('jumps to a student attendance log with the id query param', async () => {
		const store = await freshStore();
		store.openPalette();
		await vi.waitFor(() => expect(store.students).toHaveLength(3));

		const item = store.allItems().find((i) => i.id === 'student-s1')!;
		expect(item.label).toBe('Juan Dela Cruz');
		store.run(item);
		expect(mocks.goto).toHaveBeenCalledWith('/students?student=s1');
	});

	it('surfaces a student load failure without crashing', async () => {
		mocks.listStudents.mockRejectedValue(new Error('db down'));
		const store = await freshStore();
		store.openPalette();
		await vi.waitFor(() => expect(store.studentsFailed).toBe(true));
		expect(store.open).toBe(true);
	});
});
