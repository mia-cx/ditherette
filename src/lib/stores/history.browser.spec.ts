import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { scheduleProcessing } from '$lib/processing/client';
import { colorSpace, ditherSettings, outputSettings, updateDitherSettings } from './app';
import { addEffect, effectLayers, updateEffect } from './effects';
import {
	MAX_HISTORY,
	SETTLE_MS,
	canRedo,
	canUndo,
	redo,
	startSettingsHistory,
	undo
} from './history';

vi.mock('$lib/processing/client', { spy: true });

let stop: () => void;

beforeEach(() => {
	vi.useFakeTimers();
	colorSpace.set('oklab');
	updateDitherSettings({ algorithm: 'none', strength: 100 });
	outputSettings.set({ ...outputSettings.get(), crop: undefined });
	effectLayers.set([]);
	stop = startSettingsHistory();
});

afterEach(() => {
	stop();
	vi.useRealTimers();
});

const pointer = (type: string, pointerId = 1) =>
	window.dispatchEvent(new PointerEvent(type, { pointerId }));
const wait = (ms: number) => vi.advanceTimersByTimeAsync(ms);
const settle = () => wait(SETTLE_MS);

describe('settings history', () => {
	it('records a whole drag as one step', async () => {
		pointer('pointerdown');
		for (let strength = 99; strength >= 40; strength--) {
			updateDitherSettings({ strength });
			await wait(10);
		}
		await settle();
		expect(canUndo.get()).toBe(false);
		pointer('pointerup');
		await settle();

		undo();
		expect(ditherSettings.get().strength).toBe(100);
		expect(canUndo.get()).toBe(false);
		redo();
		expect(ditherSettings.get().strength).toBe(40);
	});

	it('keeps a two-pointer gesture in one step until every pointer lifts', async () => {
		pointer('pointerdown', 1);
		updateDitherSettings({ strength: 80 });
		pointer('pointerdown', 2);
		updateDitherSettings({ strength: 60 });
		pointer('pointerup', 1);
		await settle();
		updateDitherSettings({ strength: 40 });
		pointer('pointerup', 2);
		await settle();

		undo();
		expect(ditherSettings.get().strength).toBe(100);
		expect(canUndo.get()).toBe(false);
	});

	it('commits a gesture whose pointer release was lost to a blur', async () => {
		pointer('pointerdown');
		updateDitherSettings({ strength: 80 });
		window.dispatchEvent(new Event('blur'));
		await settle();
		expect(canUndo.get()).toBe(true);
	});

	it('merges quick repeats to one control and separates other controls', async () => {
		const [first, second] = [addEffect('exposure'), addEffect('exposure')];
		await settle();
		updateDitherSettings({ strength: 90 });
		await wait(50);
		updateDitherSettings({ strength: 80 });
		await wait(50);
		colorSpace.set('srgb');
		await wait(50);
		updateEffect(first.id, { effect: 'exposure', enabled: true, stops: 1 });
		await wait(50);
		updateEffect(second.id, { effect: 'exposure', enabled: true, stops: 2 });
		await settle();

		const stops = () => effectLayers.get().map(({ step }) => 'stops' in step && step.stops);
		undo();
		expect(stops()).toEqual([1, 0]);
		undo();
		expect(stops()).toEqual([0, 0]);
		undo();
		expect(colorSpace.get()).toBe('oklab');
		expect(ditherSettings.get().strength).toBe(80);
		undo();
		expect(ditherSettings.get().strength).toBe(100);
	});

	it('steps back through settled edits, and a new edit drops the redo branch', async () => {
		updateDitherSettings({ algorithm: 'floyd-steinberg' });
		await settle();
		colorSpace.set('cielab');
		await settle();

		undo();
		expect(colorSpace.get()).toBe('oklab');
		undo();
		expect(ditherSettings.get().algorithm).toBe('none');
		// Restoring settings is not itself an edit.
		await settle();
		expect(canRedo.get()).toBe(true);

		updateDitherSettings({ algorithm: 'atkinson' });
		undo();
		expect(ditherSettings.get().algorithm).toBe('none');
		redo();
		expect(ditherSettings.get().algorithm).toBe('atkinson');
		expect(canRedo.get()).toBe(false);
	});

	it('reprocesses a restore at once instead of after the slider debounce', async () => {
		updateDitherSettings({ strength: 40 });
		await settle();
		vi.mocked(scheduleProcessing).mockClear();
		undo();
		expect(scheduleProcessing).toHaveBeenLastCalledWith(0);
	});

	it('skips edits that end where they began', async () => {
		updateDitherSettings({ strength: 50 });
		updateDitherSettings({ strength: 100 });
		await settle();
		expect(canUndo.get()).toBe(false);
	});

	it('starts over after a crop, so no size is restored onto another frame', async () => {
		updateDitherSettings({ algorithm: 'atkinson' });
		await settle();
		outputSettings.set({ ...outputSettings.get(), crop: { x: 0, y: 0, width: 8, height: 8 } });
		undo();
		expect(canUndo.get()).toBe(false);
		expect(ditherSettings.get().algorithm).toBe('atkinson');
	});

	it(`keeps the last ${MAX_HISTORY} steps`, async () => {
		for (let strength = 0; strength <= MAX_HISTORY; strength++) {
			updateDitherSettings({ strength });
			await settle();
		}
		for (let step = 0; step < MAX_HISTORY; step++) undo();
		expect(canUndo.get()).toBe(false);
		expect(ditherSettings.get().strength).toBe(0);
	});
});
