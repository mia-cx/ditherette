import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { colorSpace, ditherSettings, outputSettings, updateDitherSettings } from './app';
import {
	MAX_HISTORY,
	SETTLE_MS,
	canRedo,
	canUndo,
	redo,
	startSettingsHistory,
	undo
} from './history';

let stop: () => void;

beforeEach(() => {
	vi.useFakeTimers();
	colorSpace.set('oklab');
	updateDitherSettings({ algorithm: 'none', strength: 100 });
	outputSettings.set({ ...outputSettings.get(), crop: undefined });
	stop = startSettingsHistory();
});

afterEach(() => {
	stop();
	vi.useRealTimers();
});

const pointer = (type: string) => window.dispatchEvent(new PointerEvent(type));
const settle = () => vi.advanceTimersByTime(SETTLE_MS);

describe('settings history', () => {
	it('records a whole drag as one step', () => {
		pointer('pointerdown');
		for (let strength = 99; strength >= 40; strength--) updateDitherSettings({ strength });
		settle();
		expect(canUndo.get()).toBe(false);
		pointer('pointerup');
		settle();

		undo();
		expect(ditherSettings.get().strength).toBe(100);
		expect(canUndo.get()).toBe(false);
		redo();
		expect(ditherSettings.get().strength).toBe(40);
	});

	it('steps back through settled edits, and a new edit drops the redo branch', () => {
		updateDitherSettings({ algorithm: 'floyd-steinberg' });
		settle();
		colorSpace.set('cielab');
		settle();

		undo();
		expect(colorSpace.get()).toBe('oklab');
		undo();
		expect(ditherSettings.get().algorithm).toBe('none');
		// Restoring settings is not itself an edit.
		settle();
		expect(canRedo.get()).toBe(true);

		updateDitherSettings({ algorithm: 'atkinson' });
		undo();
		expect(ditherSettings.get().algorithm).toBe('none');
		redo();
		expect(ditherSettings.get().algorithm).toBe('atkinson');
		expect(canRedo.get()).toBe(false);
	});

	it('skips edits that end where they began', () => {
		updateDitherSettings({ strength: 50 });
		updateDitherSettings({ strength: 100 });
		settle();
		expect(canUndo.get()).toBe(false);
	});

	it('starts over after a crop, so no size is restored onto another frame', () => {
		updateDitherSettings({ algorithm: 'atkinson' });
		settle();
		outputSettings.set({ ...outputSettings.get(), crop: { x: 0, y: 0, width: 8, height: 8 } });
		undo();
		expect(canUndo.get()).toBe(false);
		expect(ditherSettings.get().algorithm).toBe('atkinson');
	});

	it(`keeps the last ${MAX_HISTORY} steps`, () => {
		for (let strength = 0; strength <= MAX_HISTORY; strength++) {
			updateDitherSettings({ strength });
			settle();
		}
		for (let step = 0; step < MAX_HISTORY; step++) undo();
		expect(canUndo.get()).toBe(false);
		expect(ditherSettings.get().strength).toBe(0);
	});
});
