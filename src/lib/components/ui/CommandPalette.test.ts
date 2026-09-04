import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import CommandPalette from './CommandPalette.svelte';

const mocks = vi.hoisted(() => ({
	goto: vi.fn(),
	listStudents: vi.fn()
}));

vi.mock('$app/navigation', () => ({ goto: mocks.goto }));
vi.mock('$lib/db-rust', () => ({ listStudents: mocks.listStudents }));

import { commandPaletteStore } from '$lib/stores/command-palette.svelte';

const students = [
	{ id: 's1', name: 'Juan Dela Cruz', createdAt: '2026-06-01' },
	{ id: 's2', name: 'Maria Santos', createdAt: '2026-06-01' }
];

function pressCtrlK() {
	window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }));
}

describe('CommandPalette', () => {
	beforeEach(() => {
		vi.clearAllMocks();
		mocks.listStudents.mockResolvedValue(students);
		commandPaletteStore.closePalette();
		commandPaletteStore.setCardReaderArmed(false);
	});

	afterEach(() => {
		commandPaletteStore.closePalette();
		commandPaletteStore.setCardReaderArmed(false);
	});

	it('is hidden by default', () => {
		render(CommandPalette);
		expect(screen.queryByLabelText('Search commands')).not.toBeInTheDocument();
	});

	it('opens with Ctrl+K and focuses the query box', async () => {
		render(CommandPalette);
		pressCtrlK();

		const input = await screen.findByLabelText('Search commands');
		expect(input).toBeInTheDocument();
		expect(document.activeElement).toBe(input);
	});

	it('lists static page items on open', async () => {
		render(CommandPalette);
		pressCtrlK();

		expect(await screen.findByRole('option', { name: /SF2 Reports/ })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: /Take Attendance/ })).toBeInTheDocument();
		expect(screen.getByRole('option', { name: /Class List/ })).toBeInTheDocument();
	});

	it('filters students by fuzzy name and runs the selected one with Enter', async () => {
		render(CommandPalette);
		pressCtrlK();

		const input = await screen.findByLabelText('Search commands');
		fireEvent.input(input, { target: { value: 'juan' } });

		await screen.findByRole('option', { name: /Juan Dela Cruz/ });
		expect(screen.queryByRole('option', { name: /Maria Santos/ })).not.toBeInTheDocument();

		fireEvent.keyDown(input, { key: 'Enter' });
		await vi.waitFor(() => expect(mocks.goto).toHaveBeenCalledWith('/students?student=s1'));
		expect(commandPaletteStore.open).toBe(false);
	});

	it('navigates to reports when Enter is pressed on the first result', async () => {
		render(CommandPalette);
		pressCtrlK();

		const input = await screen.findByLabelText('Search commands');
		fireEvent.input(input, { target: { value: 'reports' } });
		fireEvent.keyDown(input, { key: 'Enter' });

		await vi.waitFor(() => expect(mocks.goto).toHaveBeenCalledWith('/reports'));
	});

	it('closes with Escape and restores focus', async () => {
		render(CommandPalette);
		pressCtrlK();
		const input = await screen.findByLabelText('Search commands');

		fireEvent.keyDown(input, { key: 'Escape' });
		expect(commandPaletteStore.open).toBe(false);
		expect(screen.queryByLabelText('Search commands')).not.toBeInTheDocument();
	});

	it('shows an empty state when nothing matches', async () => {
		render(CommandPalette);
		pressCtrlK();

		const input = await screen.findByLabelText('Search commands');
		fireEvent.input(input, { target: { value: 'zzz-no-such-thing' } });

		expect(await screen.findByText(/No results for/)).toBeInTheDocument();
	});

	it('does not open with Ctrl+K while the card reader is armed', async () => {
		commandPaletteStore.setCardReaderArmed(true);
		render(CommandPalette);
		pressCtrlK();

		expect(screen.queryByLabelText('Search commands')).not.toBeInTheDocument();
		expect(commandPaletteStore.open).toBe(false);
	});

	it('closes when the backdrop is clicked', async () => {
		render(CommandPalette);
		pressCtrlK();
		await screen.findByLabelText('Search commands');

		fireEvent.click(screen.getByLabelText('Close command palette'));
		expect(commandPaletteStore.open).toBe(false);
	});
});
