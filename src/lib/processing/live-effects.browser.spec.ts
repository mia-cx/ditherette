import { describe, expect, it } from 'vitest';
import { compileEffects, indexColours, type Effect } from 'ditherette';
import { packageEffectContext, packageProcessRequest } from './package-adapter';
import { resolveRecipes } from './recipes';
import type { EnabledPaletteColor, ProcessingSettings } from './types';
import { initializePackageProcessor, ProcessorWorkerPipeline } from './worker-pipeline';

const WIDTH = 48;
const HEIGHT = 32;

/** Gradients, repeated flats, a few near-greys, and every alpha class. */
function fixture() {
	const data = new Uint8ClampedArray(WIDTH * HEIGHT * 4);
	for (let y = 0; y < HEIGHT; y++) {
		for (let x = 0; x < WIDTH; x++) {
			const offset = (y * WIDTH + x) * 4;
			const flat = x < 8;
			data[offset] = flat ? 200 : (x * 37 + y * 11) % 256;
			data[offset + 1] = flat ? 120 : (x * 5 + y * 23) % 256;
			data[offset + 2] = flat ? 40 : (x * 13 + y * 7) % 256;
			data[offset + 3] = y === 0 ? 0 : y === 1 ? 128 : 255;
		}
	}
	return new ImageData(data, WIDTH, HEIGHT);
}

const colour = (key: string, r: number, g: number, b: number): EnabledPaletteColor => ({
	name: key,
	key,
	rgb: { r, g, b },
	kind: 'custom',
	enabled: true
});
const palette = [
	colour('#000000', 0, 0, 0),
	colour('#FFFFFF', 255, 255, 255),
	colour('#D03030', 208, 48, 48),
	colour('#30A050', 48, 160, 80),
	colour('#3050D0', 48, 80, 208),
	colour('#E0C040', 224, 192, 64),
	colour('#808080', 128, 128, 128),
	{ name: 'Clear', key: 'transparent', kind: 'transparent', enabled: true }
] satisfies EnabledPaletteColor[];

const CHAINS: Record<string, Effect[]> = {
	tone: [
		{
			effect: 'levels',
			enabled: true,
			channel: 'rgb',
			input: { black: 0.1, white: 0.9 },
			gamma: 1.4,
			output: { black: 0.02, white: 0.98 }
		},
		{ effect: 'brightness-contrast', enabled: true, brightness: 0.1, contrast: 0.3 },
		{ effect: 'exposure', enabled: true, stops: 0.7 }
	],
	colour: [
		{ effect: 'white-balance', enabled: true, temperature: 0.3, tint: -0.2 },
		{ effect: 'hue-saturation', enabled: true, hue: 0.1, saturation: 0.3, lightness: -0.1 },
		{
			effect: 'curves',
			enabled: true,
			curves: [
				{
					kind: 'remap',
					x: { model: 'oklch', channel: 'lightness' },
					y: { model: 'oklch', channel: 'lightness' },
					points: [
						[0, 0],
						[0.5, 0.62],
						[1, 1]
					]
				}
			]
		}
	],
	'palette fit': [
		{ effect: 'exposure', enabled: true, stops: -0.4 },
		{ effect: 'recolour', enabled: true, strength: 0.8, recipe: null },
		{ effect: 'hue-saturation', enabled: true, hue: 0, saturation: 0.2, lightness: 0 }
	]
};

const settings: ProcessingSettings = {
	output: {
		width: 20,
		height: 14,
		resize: 'bilinear',
		crop: { x: 6, y: 2, width: 36, height: 26 },
		alphaMode: 'preserve',
		alphaThreshold: 0,
		matteKey: '#FFFFFF',
		lockAspect: false,
		autoSizeOnUpload: false,
		scaleFactor: 1
	},
	dither: {
		algorithm: 'floyd-steinberg',
		strength: 100,
		placement: 'everywhere',
		placementRadius: 3,
		placementThreshold: 12,
		placementSoftness: 8,
		serpentine: true,
		seed: 1,
		useColorSpace: true
	},
	colorSpace: 'oklab',
	effects: []
};

describe('live effects', () => {
	for (const [name, effects] of Object.entries(CHAINS)) {
		it(`output from forwarded compiled effects matches process with the effects: ${name}`, async () => {
			const ditherette = await initializePackageProcessor();
			const source = fixture();
			const size = { width: settings.output.width, height: settings.output.height };
			const expected = ditherette.process(
				packageProcessRequest(source, palette, { ...settings, effects }, size).request
			);
			// What the effects worker does: index once, resolve palette fit on the crop, compile.
			const image = { width: WIDTH, height: HEIGHT, data: new Uint8Array(source.data.buffer) };
			const context = packageEffectContext(palette, settings.colorSpace);
			const crop = settings.output.crop;
			const results = compileEffects(ditherette, {
				version: 1,
				colours: indexColours(image).colours,
				effects: resolveRecipes(ditherette, image, effects, context, crop),
				context
			});
			const pipeline = new ProcessorWorkerPipeline();
			pipeline.handle({ id: 1, type: 'load-source', sourceId: 'fixture', source });
			const response = await pipeline.handleAsync(
				{
					id: 2,
					type: 'process',
					sourceId: 'fixture',
					settings: { ...settings, effects },
					palette,
					settingsHash: name,
					compiledEffects: { key: name, results }
				},
				() => undefined
			);
			if (response?.type !== 'complete') throw new Error('Expected output.');
			expect(response.image.indices).toEqual(expected.indices);
		});
	}
});
