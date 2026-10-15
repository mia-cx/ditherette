import { afterEach, describe, expect, it, vi } from 'vitest';
import { packageProcessRequest } from '$lib/processing/package-adapter';
import type { Palette, ProcessingSettings } from '$lib/processing/types';

interface EngineEvent {
	key: string;
	newValue: string;
}
type EngineListener = (event: EngineEvent) => void;

/** A persistent engine whose listeners and restores are recorded per key for the tests. */
function recordingEngine() {
	const storage: Record<string, string> = {};
	const listeners = new Map<string, { listener: EngineListener; restore: () => void }>();
	const removed: string[] = [];
	const events = {
		perKey: false,
		addEventListener(key: string, listener: EngineListener, restore: () => void) {
			listeners.set(key, { listener, restore });
		},
		removeEventListener(key: string, listener: EngineListener) {
			if (listeners.get(key)?.listener === listener) listeners.delete(key);
			removed.push(key);
		}
	};
	return { storage, events, listeners, removed };
}

const outputWithMatte = (matteKey: string) => ({
	width: 512,
	height: 512,
	lockAspect: true,
	resize: 'bilinear',
	alphaMode: 'matte',
	alphaThreshold: 0,
	matteKey,
	autoSizeOnUpload: false,
	scaleFactor: 1
});

const oldCopyPalette: Palette = {
	name: 'Old copy',
	source: 'custom',
	colors: [
		{ name: 'Red', key: '#FF0000', rgb: { r: 255, g: 0, b: 0 }, kind: 'custom' },
		{ name: 'Old teal', key: '#10AE82', rgb: { r: 16, g: 174, b: 130 }, kind: 'custom' },
		{ name: 'Transparent', key: 'transparent', kind: 'transparent' }
	]
};

// Reset first: the engine lives on the persistent module instance, so it must be set on the
// copy `app` will import, not the one bound above.
async function loadApp(engine: ReturnType<typeof recordingEngine>) {
	vi.resetModules();
	const { setPersistentEngine } = await import('@nanostores/persistent');
	setPersistentEngine(engine.storage, engine.events);
	return import('./app');
}

function requestSettings(app: Awaited<ReturnType<typeof loadApp>>): ProcessingSettings {
	return {
		output: app.outputSettings.get(),
		dither: app.ditherSettings.get(),
		colorSpace: app.colorSpace.get(),
		effects: []
	};
}

const transparentSource = { width: 1, height: 1, data: new Uint8ClampedArray([0, 0, 0, 0]) };

function matteAlpha(app: Awaited<ReturnType<typeof loadApp>>) {
	const { request, warnings } = packageProcessRequest(
		transparentSource,
		app.selectedPalette.get(),
		requestSettings(app),
		{ width: 1, height: 1 }
	);
	return { alpha: request.recipe.alpha, warnings };
}

afterEach(() => {
	vi.useRealTimers();
});

describe('Wplace Teal key state', () => {
	it('keeps a Teal matte on the colour whichever palette side is active', async () => {
		const engine = recordingEngine();
		engine.storage['ditherette:active-palette'] = JSON.stringify('Old copy');
		engine.storage['ditherette:custom-palettes'] = JSON.stringify([oldCopyPalette]);
		engine.storage['ditherette:output'] = JSON.stringify(outputWithMatte('#10AE82'));
		const app = await loadApp(engine);

		expect(matteAlpha(app).alpha).toEqual({ mode: 'matte', rgb: [16, 174, 130] });

		app.activePaletteName.set('Wplace (Default)');
		const onWplace = matteAlpha(app);
		expect(onWplace.alpha).toEqual({ mode: 'matte', rgb: [16, 174, 166] });
		expect(onWplace.warnings).toEqual([]);
		expect(app.outputSettings.get().matteKey).toBe('#10AE82');

		app.activePaletteName.set('Old copy');
		expect(matteAlpha(app).alpha).toEqual({ mode: 'matte', rgb: [16, 174, 130] });
	});

	it('resolves a matte restored through the persistence listener and pageshow', async () => {
		const engine = recordingEngine();
		const app = await loadApp(engine);

		const stop = app.outputSettings.listen(() => {});
		const entry = engine.listeners.get('ditherette:output');
		expect(entry).toBeDefined();

		entry!.listener({
			key: 'ditherette:output',
			newValue: JSON.stringify(outputWithMatte('#10AE82'))
		});
		expect(matteAlpha(app).alpha).toEqual({ mode: 'matte', rgb: [16, 174, 166] });

		engine.storage['ditherette:output'] = JSON.stringify({
			...outputWithMatte('#10AEA6'),
			width: 256
		});
		entry!.restore();
		expect(matteAlpha(app).alpha).toEqual({ mode: 'matte', rgb: [16, 174, 166] });

		vi.useFakeTimers();
		stop();
		vi.advanceTimersByTime(1100); // STORE_UNMOUNT_DELAY is 1000 ms
		expect(engine.removed).toContain('ditherette:output');
	});

	it('applies Teal visibility saved under the previous key', async () => {
		const engine = recordingEngine();
		engine.storage['ditherette:palette-enabled'] = JSON.stringify({ '#10AE82': false });
		const app = await loadApp(engine);

		expect(app.selectedPalette.get().some((color) => color.name === 'Teal')).toBe(false);
	});
});
