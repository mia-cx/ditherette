import { afterEach, expect, it, vi } from 'vitest';
import { createDitherette } from 'ditherette';
import { FALLBACK_WARNING, faithfulTypeScriptFallback } from './package-fallback';
import { packageProcessRequest, packageQuantizeResult } from './package-adapter';
import { ProcessorWorkerPipeline } from './worker-pipeline';
import type { EnabledPaletteColor, ProcessingSettings } from './types';

afterEach(() => vi.unstubAllEnvs());

const palette: EnabledPaletteColor[] = [
	{
		name: 'White first',
		key: '#FFFFFF',
		rgb: { r: 255, g: 255, b: 255 },
		kind: 'custom',
		enabled: true
	},
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'custom', enabled: true },
	{
		name: 'White duplicate',
		key: '#FFFFFF-copy',
		rgb: { r: 255, g: 255, b: 255 },
		kind: 'custom',
		enabled: true
	},
	{ name: 'Clear', key: 'transparent', kind: 'transparent', enabled: true }
];
const settings: ProcessingSettings = {
	output: {
		width: 4,
		height: 1,
		resize: 'nearest',
		alphaMode: 'preserve',
		alphaThreshold: 127.9999999,
		matteKey: '#FFFFFF',
		lockAspect: false,
		autoSizeOnUpload: false,
		scaleFactor: 1
	},
	dither: {
		algorithm: 'none',
		strength: 100,
		placement: 'everywhere',
		placementRadius: 1,
		placementThreshold: 0,
		placementSoftness: 0,
		serpentine: false,
		seed: 1,
		useColorSpace: false
	},
	colorSpace: 'srgb'
};

it('admitted fallback outputs match the actual package, including crop, palette order and alpha precision', async () => {
	vi.stubEnv('DEV', true);
	vi.stubEnv('VITE_DITHERETTE_WASM_PROCESS', 'true');
	const processor = await createDitherette();
	try {
		const cases = [
			{
				source: new ImageData(new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255]), 2, 1),
				palette,
				settings
			},
			{
				source: new ImageData(
					new Uint8ClampedArray([9, 9, 9, 255, 0, 0, 0, 128, 37, 91, 211, 127, 8, 8, 8, 255]),
					4,
					1
				),
				palette,
				settings: {
					...settings,
					output: { ...settings.output, crop: { x: 1, y: 0, width: 2, height: 1 } }
				}
			},
			{
				source: new ImageData(new Uint8ClampedArray([23, 67, 189, 255, 180, 120, 40, 0]), 2, 1),
				palette: [palette[1]],
				settings
			},
			{
				source: new ImageData(new Uint8ClampedArray([0, 0, 0, 128, 37, 91, 211, 127]), 2, 1),
				palette,
				settings: { ...settings, output: { ...settings.output, alphaThreshold: 128 } }
			},
			{
				source: new ImageData(new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255]), 2, 1),
				palette: [...Array.from({ length: 256 }, () => palette[1]), palette[0]],
				settings
			},
			{
				source: new ImageData(new Uint8ClampedArray([23, 67, 189, 255, 180, 120, 40, 0]), 2, 1),
				palette: [palette[3]],
				settings
			}
		];
		for (const fixture of cases) {
			expect(faithfulTypeScriptFallback(fixture.source, fixture.palette, fixture.settings)).toBe(
				true
			);
			const mapped = packageProcessRequest(
				fixture.source,
				fixture.palette,
				fixture.settings,
				fixture.settings.output
			);
			const expected = packageQuantizeResult(
				processor.process(mapped.request),
				fixture.palette,
				mapped.warnings
			);
			const pipeline = new ProcessorWorkerPipeline();
			pipeline.handle(
				{ id: 1, type: 'load-source', sourceId: 'fixture', source: fixture.source },
				() => undefined
			);
			const actual = await pipeline.handleAsync(
				{
					id: 2,
					type: 'process',
					sourceId: 'fixture',
					palette: fixture.palette,
					settings: fixture.settings,
					settingsHash: 'fixture',
					typeScriptFallback: true
				},
				() => undefined
			);
			if (actual?.type !== 'complete') throw new Error('Expected fallback output.');
			const { indices, palette, transparentIndex, warnings } = actual.image;
			expect({ indices, palette, transparentIndex, warnings }).toEqual({
				...expected,
				warnings: [...expected.warnings, FALLBACK_WARNING]
			});
		}
	} finally {
		processor.dispose();
	}
});

it('rejects the real nearest center-tie mismatch instead of silently substituting TypeScript', async () => {
	const source = new ImageData(new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255]), 2, 1);
	const tie = { ...settings, output: { ...settings.output, width: 49 } };
	expect(faithfulTypeScriptFallback(source, palette, tie)).toBe(false);
	const processor = await createDitherette();
	try {
		const mapped = packageProcessRequest(source, palette, tie, tie.output);
		const expected = processor.process(mapped.request);
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle({ id: 1, type: 'load-source', sourceId: 'tie', source }, () => undefined);
		const actual = pipeline.handle(
			{ id: 2, type: 'process', sourceId: 'tie', palette, settings: tie, settingsHash: 'tie' },
			() => undefined
		);
		if (actual?.type !== 'complete') throw new Error('Expected TypeScript diagnostic output.');
		expect(expected.indices[24]).toBe(0);
		expect(actual.image.indices[24]).toBe(1);
	} finally {
		processor.dispose();
	}
});
