import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import AppShell from './AppShell.svelte';

// Mock SvelteKit modules
vi.mock('$app/state', () => ({
	page: {
		url: { pathname: '/' }
	}
}));

vi.mock('$app/paths', () => ({
	base: ''
}));

vi.mock('$lib/stores/settings.svelte', () => ({
	settingsStore: {
		settings: {
			quarter: '1st Quarter',
			attendanceMode: 'card_reader'
		},
		loading: false,
		error: null
	}
}));

function renderAppShell() {
	return render(AppShell, {
		props: {
			children: () => '<div>page content</div>'
		}
	});
}

describe('AppShell sidebar collapse', () => {
	it('renders the sidebar with expanded state by default', () => {
		renderAppShell();
		const sidebar = screen.getByLabelText('Primary navigation');
		expect(sidebar).toBeInTheDocument();
		// Sidebar should NOT have the collapsed class by default
		expect(sidebar.className).not.toContain('collapsed');
	});

	it('has a collapse toggle button with aria-label', () => {
		renderAppShell();
		const toggleBtn = screen.getByLabelText('Toggle sidebar');
		expect(toggleBtn).toBeInTheDocument();
	});

	it('toggles collapsed state when clicking the toggle button', async () => {
		renderAppShell();
		const toggleBtn = screen.getByLabelText('Toggle sidebar');
		const sidebar = screen.getByLabelText('Primary navigation');

		// Click to collapse
		await fireEvent.click(toggleBtn);
		expect(sidebar.className).toContain('collapsed');

		// Click to expand again
		await fireEvent.click(toggleBtn);
		expect(sidebar.className).not.toContain('collapsed');
	});

	it('hides text labels when sidebar is collapsed', async () => {
		renderAppShell();
		const toggleBtn = screen.getByLabelText('Toggle sidebar');

		// Collapse sidebar
		await fireEvent.click(toggleBtn);

		// Nav labels with text should have the hidden class
		const navLinks = screen.getAllByRole('link');
		navLinks.forEach((link) => {
			const textSpan = link.querySelector('.nav-label');
			if (textSpan) {
				expect(textSpan.className).toContain('hidden');
			}
		});
	});

	it('shows nav icons when sidebar is collapsed', async () => {
		renderAppShell();
		const toggleBtn = screen.getByLabelText('Toggle sidebar');

		// Collapse sidebar
		await fireEvent.click(toggleBtn);

		// Icon containers should still be visible
		const iconSpans = document.querySelectorAll('.nav-icon');
		expect(iconSpans.length).toBeGreaterThan(0);
		iconSpans.forEach((icon) => {
			expect(icon.className).not.toContain('hidden');
		});
	});

	it('hides the logo text when sidebar is collapsed', async () => {
		renderAppShell();
		const toggleBtn = screen.getByLabelText('Toggle sidebar');

		// Collapse sidebar
		await fireEvent.click(toggleBtn);

		// Logo text "EES AMS" should be hidden
		const logoText = screen.getByText('EES AMS');
		expect(logoText.className).toContain('hidden');
	});

	it('hides the footer when sidebar is collapsed', async () => {
		renderAppShell();
		const toggleBtn = screen.getByLabelText('Toggle sidebar');

		// Collapse sidebar
		await fireEvent.click(toggleBtn);

		// Footer should have hidden class
		const footer = document.querySelector('.sidebar-footer');
		expect(footer).not.toBeNull();
		expect(footer!.className).toContain('hidden');
	});
});
