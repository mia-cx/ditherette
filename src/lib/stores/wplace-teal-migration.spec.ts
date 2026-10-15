import { describe, expect, it, vi } from 'vitest';
import { packageProcessRequest } from '$lib/processing/package-adapter';
import type { Palette, ProcessingSettings } from '$lib/processing/types';

const noopEvents = {
	addEventListener() {},
	removeEventListener() {},
	perKey: false
};

const outputWithMatte = {
	width: 512,
	height: 512,
	lockAspect: true,
	resize: 'bilinear',
	alphaMode: 'matte',
	alphaThreshold: 0,
	matteKey: '#10AE82',
	autoSizeOnUpload: false,
	scaleFactor: 1
};

const oldTealPalette: Palette = {
	name: 'Old copy',
	source: 'custom',
	colors: [
		{ name: 'Old teal', key: '#10AE82', rgb: { r: 16, g: 174, b: 130 }, kind: 'custom' },
		{ name: 'Transparent', key: 'transparent', kind: 'transparent' }
	]
};

// Reset first: the engine lives on the persistent module instance, so it must be set on the
// copy `app` will import, not the one bound above.
async function loadApp(storage: Record<string, string>) {
	vi.resetModules();
	const { setPersistentEngine } = await import('@nanostores/persistent');
	setPersistentEngine(storage, noopEvents);
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

describe('Wplace Teal key migration', () => {
	it('carries a saved Teal matte to the corrected colour', async () => {
		const app = await loadApp({ 'ditherette:output': JSON.stringify(outputWithMatte) });

		expect(app.outputSettings.get().matteKey).toBe('#10AEA6');

		const { request, warnings } = packageProcessRequest(
			transparentSource,
			app.selectedPalette.get(),
			requestSettings(app),
			{ width: 1, height: 1 }
		);
		expect(request.recipe.alpha).toEqual({ mode: 'matte', rgb: [16, 174, 166] });
		expect(warnings).toEqual([]);
		expect(request.palette).toContainEqual({ kind: 'color', rgb: [16, 174, 166] });
		expect(request.palette).not.toContainEqual({ kind: 'color', rgb: [16, 174, 130] });
	});

	it('leaves the matte when the active custom palette holds the previous colour', async () => {
		const app = await loadApp({
			'ditherette:output': JSON.stringify(outputWithMatte),
			'ditherette:custom-palettes': JSON.stringify([oldTealPalette]),
			'ditherette:active-palette': JSON.stringify('Old copy')
		});

		expect(app.outputSettings.get().matteKey).toBe('#10AE82');
	});

	it('applies Teal visibility saved under the previous key', async () => {
		const app = await loadApp({
			'ditherette:palette-enabled': JSON.stringify({ '#10AE82': false })
		});

		expect(app.selectedPalette.get().some((color) => color.name === 'Teal')).toBe(false);
	});
});
