import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { scheduleProcessing } from '$lib/processing/client';
import type { SourceMeta } from './app';
import {
	activePaletteName,
	colorSpace,
	createCustomPalette,
	ditherSettings,
	outputSettings,
	paletteEnabled,
	previewSettings,
	setAllPaletteColors,
	sourceMeta,
	updateDitherSettings,
	updateOutputSettings,
	updatePreviewSettings
} from './app';
import { addEffect, effectLayers, updateEffect } from './effects';
import {
	HISTORY_LIMIT,
	canRedo,
	canUndo,
	editsText,
	historyCommand,
	redo,
	startHistory,
	undo
} from './history';

vi.mock('$lib/processing/client', () => ({ scheduleProcessing: vi.fn() }));

const SETTLE = 400;
const settle = () => vi.advanceTimersByTimeAsync(SETTLE);
const microtasks = () => vi.advanceTimersByTimeAsync(0);
const pointer = (type: 'pointerdown' | 'pointerup') => window.dispatchEvent(new Event(type));

const source = (name: string): SourceMeta => ({
	name,
	width: 8,
	height: 8,
	type: 'image/png',
	updatedAt: 1
});

let stop: () => void;

beforeEach(() => {
	vi.useFakeTimers();
	vi.stubGlobal('window', new EventTarget());
	vi.mocked(scheduleProcessing).mockClear();
	outputSettings.set({ ...outputSettings.get(), width: 512, height: 512, crop: undefined });
	ditherSettings.set({ ...ditherSettings.get(), algorithm: 'none', strength: 100 });
	colorSpace.set('oklab');
	effectLayers.set([]);
	activePaletteName.set('Wplace');
	sourceMeta.set(undefined);
	stop = startHistory();
});

afterEach(() => {
	stop();
	vi.useRealTimers();
	vi.unstubAllGlobals();
});

describe('undo and redo', () => {
	it('steps output, dither, colour space, effects, palette and enabled state back and forth', async () => {
		updateOutputSettings({ resize: 'nearest' });
		await settle();
		updateDitherSettings({ algorithm: 'floyd-steinberg' });
		await settle();
		colorSpace.set('srgb');
		await settle();
		const layer = addEffect('exposure');
		await settle();
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 1 });
		await settle();
		createCustomPalette('Mine');
		await settle();
		setAllPaletteColors(false);
		await settle();

		for (let count = 0; count < 7; count++) undo();
		expect(outputSettings.get().resize).not.toBe('nearest');
		expect(ditherSettings.get().algorithm).toBe('none');
		expect(colorSpace.get()).toBe('oklab');
		expect(effectLayers.get()).toEqual([]);
		expect(activePaletteName.get()).toBe('Wplace');
		expect(canUndo.get()).toBe(false);

		for (let count = 0; count < 7; count++) redo();
		expect(outputSettings.get().resize).toBe('nearest');
		expect(ditherSettings.get().algorithm).toBe('floyd-steinberg');
		expect(colorSpace.get()).toBe('srgb');
		expect(activePaletteName.get()).toBe('Mine');
		expect(Object.values(paletteEnabled.get()).some((enabled) => !enabled)).toBe(true);
		expect(canRedo.get()).toBe(false);
	});

	it('undoes a pending edit without waiting for it to settle, and reprocesses at once', async () => {
		updateDitherSettings({ strength: 40 });
		await microtasks();
		expect(canUndo.get()).toBe(true);
		undo();
		expect(ditherSettings.get().strength).toBe(100);
		expect(scheduleProcessing).toHaveBeenLastCalledWith(0);
		redo();
		expect(ditherSettings.get().strength).toBe(40);
	});

	it('toggles between two states for A/B comparison', async () => {
		updateDitherSettings({ algorithm: 'atkinson' });
		await settle();
		const seen: string[] = [];
		for (let count = 0; count < 3; count++) {
			undo();
			seen.push(ditherSettings.get().algorithm);
			redo();
			seen.push(ditherSettings.get().algorithm);
		}
		expect(seen).toEqual(['none', 'atkinson', 'none', 'atkinson', 'none', 'atkinson']);
	});

	it('drops redo when a new edit follows an undo', async () => {
		updateDitherSettings({ strength: 40 });
		await settle();
		undo();
		expect(canRedo.get()).toBe(true);
		updateDitherSettings({ strength: 60 });
		await settle();
		expect(canRedo.get()).toBe(false);
		redo();
		expect(ditherSettings.get().strength).toBe(60);
	});

	it('does nothing with an empty history', () => {
		undo();
		redo();
		expect(scheduleProcessing).not.toHaveBeenCalled();
	});
});

describe('what counts as one entry', () => {
	it('keeps a slider drag to one entry however many ticks it emits', async () => {
		pointer('pointerdown');
		for (let strength = 99; strength >= 20; strength--) {
			updateDitherSettings({ strength });
			await vi.advanceTimersByTimeAsync(SETTLE * 2);
		}
		expect(canUndo.get()).toBe(true);
		pointer('pointerup');
		await settle();
		undo();
		expect(ditherSettings.get().strength).toBe(100);
		expect(canUndo.get()).toBe(false);
	});

	it('merges keyboard steps on one field but separates different fields', async () => {
		updateDitherSettings({ strength: 90 });
		await vi.advanceTimersByTimeAsync(50);
		updateDitherSettings({ strength: 80 });
		await vi.advanceTimersByTimeAsync(50);
		updateDitherSettings({ serpentine: false });
		await settle();

		undo();
		expect(ditherSettings.get()).toMatchObject({ strength: 80, serpentine: true });
		undo();
		expect(ditherSettings.get().strength).toBe(100);
		expect(canUndo.get()).toBe(false);
	});

	it('records one entry for an action that sets several stores', async () => {
		createCustomPalette('Mine');
		await settle();
		undo();
		expect(activePaletteName.get()).toBe('Wplace');
		expect(canUndo.get()).toBe(false);
	});

	it('skips edits that change nothing', async () => {
		updateDitherSettings({ strength: 100 });
		setAllPaletteColors(true);
		await settle();
		expect(canUndo.get()).toBe(false);
	});

	it('leaves preview pan and zoom out of history', async () => {
		updatePreviewSettings({ zoom: 3, panX: 10 });
		await settle();
		expect(canUndo.get()).toBe(false);
		expect(previewSettings.get().zoom).toBe(3);
	});

	it('keeps only the newest entries', async () => {
		for (let width = 1; width <= HISTORY_LIMIT + 20; width++) {
			updateOutputSettings({ width });
			await settle();
		}
		let undone = 0;
		while (canUndo.get()) {
			undo();
			undone++;
		}
		expect(undone).toBe(HISTORY_LIMIT);
		expect(outputSettings.get().width).toBe(20);
	});
});

describe('boundaries', () => {
	it('forgets history when the source changes', async () => {
		updateDitherSettings({ strength: 40 });
		await settle();
		sourceMeta.set(source('a.png'));
		updateOutputSettings({ width: 64, height: 64 });
		await settle();
		expect(canUndo.get()).toBe(false);
		undo();
		expect(outputSettings.get().width).toBe(64);
	});

	it('forgets history when the crop changes, so undo never shows a stale frame', async () => {
		updateDitherSettings({ strength: 40 });
		await settle();
		updateOutputSettings({ crop: { x: 0, y: 0, width: 4, height: 4 }, width: 4, height: 4 });
		await settle();
		expect(canUndo.get()).toBe(false);
		updateOutputSettings({ width: 8 });
		await settle();
		undo();
		expect(outputSettings.get()).toMatchObject({ width: 4, crop: { width: 4 } });
	});
});

describe('shortcuts', () => {
	const press = (key: string, modifiers: Partial<Parameters<typeof historyCommand>[0]> = {}) => ({
		key,
		shiftKey: false,
		altKey: false,
		metaKey: false,
		ctrlKey: false,
		...modifiers
	});

	it('maps Cmd+Z and Cmd+Shift+Z on macOS', () => {
		expect(historyCommand(press('z', { metaKey: true }), true)).toBe('undo');
		expect(historyCommand(press('Z', { metaKey: true, shiftKey: true }), true)).toBe('redo');
		expect(historyCommand(press('z', { ctrlKey: true }), true)).toBeUndefined();
		expect(historyCommand(press('y', { metaKey: true }), true)).toBeUndefined();
	});

	it('maps Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y elsewhere', () => {
		expect(historyCommand(press('z', { ctrlKey: true }), false)).toBe('undo');
		expect(historyCommand(press('Z', { ctrlKey: true, shiftKey: true }), false)).toBe('redo');
		expect(historyCommand(press('y', { ctrlKey: true }), false)).toBe('redo');
		expect(historyCommand(press('z', { metaKey: true }), false)).toBeUndefined();
		expect(historyCommand(press('z', { ctrlKey: true, altKey: true }), false)).toBeUndefined();
		expect(historyCommand(press('z'), false)).toBeUndefined();
	});

	it('finds no text field without a DOM target', () => {
		expect(editsText(null)).toBe(false);
	});
});
