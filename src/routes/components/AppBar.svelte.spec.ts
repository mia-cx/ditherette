import { beforeEach, expect, it, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
import { ditherSettings, updateDitherSettings } from '$lib/stores/app';
import { startSettingsHistory } from '$lib/stores/history';
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
	ditherSettings.set({ ...ditherSettings.get(), algorithm: 'none' });
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

it('undoes and redoes settings with Ctrl+Z, Ctrl+Shift+Z, and Ctrl+Y', async () => {
	const stop = startSettingsHistory();
	try {
		await render(AppBar, {
			hasImage: true,
			studio: false,
			onChooseImage: () => {},
			onClear: () => {}
		});
		updateDitherSettings({ algorithm: 'atkinson' });
		await userEvent.keyboard('{Control>}z{/Control}');
		expect(ditherSettings.get().algorithm).toBe('none');
		await userEvent.keyboard('{Control>}{Shift>}z{/Shift}{/Control}');
		expect(ditherSettings.get().algorithm).toBe('atkinson');
		await userEvent.keyboard('{Control>}z{/Control}');
		await userEvent.keyboard('{Control>}y{/Control}');
		expect(ditherSettings.get().algorithm).toBe('atkinson');
	} finally {
		stop();
	}
});

it('leaves Ctrl+Z to text editors and handled keys, and undoes from other controls', async () => {
	const stop = startSettingsHistory();
	const fields = document.body.appendChild(document.createElement('div'));
	fields.innerHTML =
		'<input type="number"><div contenteditable><span tabindex="-1">text</span></div><input type="checkbox">';
	const handled = (event: KeyboardEvent) => event.preventDefault();
	const undoKey = () => userEvent.keyboard('{Control>}z{/Control}');
	try {
		await render(AppBar, {
			hasImage: true,
			studio: false,
			onChooseImage: () => {},
			onClear: () => {}
		});
		updateDitherSettings({ algorithm: 'atkinson' });
		for (const selector of ['input[type=number]', 'span']) {
			focus(fields.querySelector(selector)!);
			await undoKey();
			expect(ditherSettings.get().algorithm).toBe('atkinson');
		}
		focus(fields.querySelector('input[type=checkbox]')!);
		window.addEventListener('keydown', handled, true);
		await undoKey();
		expect(ditherSettings.get().algorithm).toBe('atkinson');
		window.removeEventListener('keydown', handled, true);
		await undoKey();
		expect(ditherSettings.get().algorithm).toBe('none');
	} finally {
		window.removeEventListener('keydown', handled, true);
		fields.remove();
		stop();
	}
});
