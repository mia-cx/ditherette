import { beforeEach, expect, it, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { ditherSettings } from '$lib/stores/app';
import { startHistory } from '$lib/stores/history';
import AppBar from './AppBar.svelte';
import { previewCommands } from './preview-commands';

const commands = {
	zoomIn: vi.fn(),
	zoomOut: vi.fn(),
	fit: vi.fn(),
	actualSize: vi.fn(),
	toggleCrop: vi.fn(),
	clearCrop: vi.fn()
};

beforeEach(() => {
	previewCommands.set(commands);
	ditherSettings.set({ ...ditherSettings.get(), algorithm: 'none', strength: 100 });
	for (const command of Object.values(commands)) command.mockClear();
});

const focus = (element: Element) => (element as HTMLElement).focus();

it('opens with the command shortcut and drives the preview with bare keys', async () => {
	const onChooseImage = vi.fn();
	await render(AppBar, { hasImage: true, studio: false, onChooseImage, onClear: () => {} });
	await userEvent.keyboard('{Control>}o{/Control}');
	await userEvent.keyboard('{Meta>}o{/Meta}');
	expect(onChooseImage).toHaveBeenCalledTimes(1);
	await userEvent.keyboard('0');
	await userEvent.keyboard('=');
	expect(commands.fit).toHaveBeenCalledOnce();
	expect(commands.zoomIn).toHaveBeenCalledOnce();
});

it('ignores view keys while there is no image', async () => {
	await render(AppBar, {
		hasImage: false,
		studio: false,
		onChooseImage: () => {},
		onClear: () => {}
	});
	await userEvent.keyboard('0');
	expect(commands.fit).not.toHaveBeenCalled();
});

it('sets the dither algorithm from Image > Dither', async () => {
	await render(AppBar, {
		hasImage: true,
		studio: false,
		onChooseImage: () => {},
		onClear: () => {}
	});
	focus(page.getByRole('menuitem', { name: 'Image', exact: true }).element());
	await userEvent.keyboard('{Enter}');
	focus(page.getByRole('menuitem', { name: 'Dither', exact: true }).element());
	await userEvent.keyboard('{ArrowRight}');
	focus(page.getByRole('menuitemradio', { name: 'Sierra Lite' }).element());
	await userEvent.keyboard('{Enter}');
	await expect.poll(() => ditherSettings.get().algorithm).toBe('sierra-lite');
});

const renderBar = () =>
	render(AppBar, { hasImage: true, studio: false, onChooseImage: () => {}, onClear: () => {} });

/** A committed edit: history settles a moment after the last change. */
const commitStrength = async (strength: number) => {
	ditherSettings.set({ ...ditherSettings.get(), strength });
	await new Promise((done) => setTimeout(done, 500));
};

it('undoes and redoes settings with the keyboard', async () => {
	const stop = startHistory();
	try {
		await renderBar();
		await commitStrength(40);
		await userEvent.keyboard('{Control>}z{/Control}');
		await userEvent.keyboard('{Meta>}z{/Meta}');
		expect(ditherSettings.get().strength).toBe(100);
		await userEvent.keyboard('{Control>}{Shift>}z{/Shift}{/Control}');
		await userEvent.keyboard('{Meta>}{Shift>}z{/Shift}{/Meta}');
		expect(ditherSettings.get().strength).toBe(40);
		await userEvent.keyboard('{Control>}z{/Control}');
		await userEvent.keyboard('{Control>}y{/Control}');
		expect(ditherSettings.get().strength).toBe(40);
	} finally {
		stop();
	}
});

it('leaves undo to text fields and keeps it for sliders', async () => {
	const stop = startHistory();
	try {
		await renderBar();
		await commitStrength(40);
		const field = document.body.appendChild(document.createElement('input'));
		field.type = 'number';
		field.focus();
		await userEvent.keyboard('{Control>}z{/Control}{Meta>}z{/Meta}');
		expect(ditherSettings.get().strength).toBe(40);
		field.remove();

		const slider = document.body.appendChild(document.createElement('input'));
		slider.type = 'range';
		slider.focus();
		await userEvent.keyboard('{Control>}z{/Control}{Meta>}z{/Meta}');
		expect(ditherSettings.get().strength).toBe(100);
		slider.remove();
	} finally {
		stop();
	}
});
