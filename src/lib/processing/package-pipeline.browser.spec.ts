import { describe, expect, it } from 'vitest';
import { decodeBlob } from './image-decode';
import { encodeIndexedPng } from './png';
import { processedToImageData } from './render';
import {
	validateProcessedImage,
	validateSourceImageRecord,
	validateWorkerRequest,
	validateWorkerResponse
} from './schemas';
import type {
	ColorSpaceId,
	DitherId,
	EnabledPaletteColor,
	ProcessedImage,
	ProcessingSettings
} from './types';
import { compileEffects } from 'ditherette';
import { initializePackageProcessor, ProcessorWorkerPipeline } from './worker-pipeline';

const uploaded: ProcessedImage = {
	width: 4,
	height: 2,
	indices: new Uint8Array([3, 0, 1, 3, 3, 2, 1, 3]),
	palette: [
		{
			name: 'Ink',
			key: '#000000',
			rgb: { r: 0, g: 0, b: 0 },
			kind: 'custom',
			tags: ['fixture'],
			enabled: true
		},
		{
			name: 'Paper',
			key: '#FFFFFF',
			rgb: { r: 255, g: 255, b: 255 },
			kind: 'custom',
			enabled: true
		},
		{ name: 'Clear', key: 'transparent', kind: 'transparent', enabled: true },
		{
			name: 'Outside crop',
			key: '#FF0000',
			rgb: { r: 255, g: 0, b: 0 },
			kind: 'custom',
			enabled: true
		}
	],
	transparentIndex: 2,
	warnings: [],
	settingsHash: 'upload-fixture',
	updatedAt: 0
};
const settings: ProcessingSettings = {
	output: {
		width: 4,
		height: 2,
		resize: 'nearest',
		crop: { x: 1, y: 0, width: 2, height: 2 },
		alphaMode: 'preserve',
		alphaThreshold: 0,
		matteKey: '#FFFFFF',
		lockAspect: false,
		autoSizeOnUpload: false,
		scaleFactor: 1
	},
	dither: {
		algorithm: 'none',
		strength: 100,
		placement: 'everywhere',
		placementMinGradientSize: 24,
		serpentine: true,
		seed: 1,
		useColorSpace: false
	},
	colorSpace: 'srgb',
	effects: []
};

describe('installed package website integration', () => {
	it('preserves the website byte-field strength at a palette decision boundary', async () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle({
			id: 1,
			type: 'load-source',
			sourceId: 'strength',
			source: new ImageData(new Uint8ClampedArray([154, 154, 154, 255]), 1, 1)
		});
		for (const [strength, expected] of [
			[100, 0],
			[50, 1]
		]) {
			const modeSettings = {
				...settings,
				output: { ...settings.output, width: 1, height: 1, crop: undefined },
				dither: { ...settings.dither, algorithm: 'bayer-2', strength }
			} satisfies ProcessingSettings;
			const response = await pipeline.handleAsync(
				{
					id: 2,
					type: 'process',
					sourceId: 'strength',
					settings: modeSettings,
					palette: uploaded.palette.slice(0, 2),
					settingsHash: `strength-${strength}`
				},
				() => undefined
			);
			if (response?.type !== 'complete') throw new Error('Expected field output.');
			// Bayer(0,0) is -3/8. The website's scale 96 gives 118 at 100%, 136 at 50%.
			expect(response.image.indices).toEqual(new Uint8Array([expected]));
		}
	});

	it('runs enabled effect steps before quantization', async () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle({
			id: 1,
			type: 'load-source',
			sourceId: 'effects',
			source: new ImageData(new Uint8ClampedArray([40, 40, 40, 255]), 1, 1)
		});
		const indices = [];
		const ditherette = await initializePackageProcessor();
		for (const effects of [[], [{ effect: 'exposure', enabled: true, stops: 4 }] as const]) {
			// The one colour, compiled as the effects worker would.
			const results = compileEffects(ditherette, {
				version: 1,
				colours: new Uint32Array([0x282828]),
				effects
			});
			const response = await pipeline.handleAsync(
				{
					id: 2,
					type: 'process',
					sourceId: 'effects',
					settings: {
						...settings,
						output: { ...settings.output, width: 1, height: 1, crop: undefined },
						effects
					},
					palette: uploaded.palette.slice(0, 2),
					settingsHash: `effects-${effects.length}`,
					compiledEffects: { key: `effects-${effects.length}`, results }
				},
				() => undefined
			);
			if (response?.type !== 'complete') throw new Error('Expected effect output.');
			indices.push(response.image.indices[0]);
		}
		// Dark grey matches black; four stops brighter it matches white.
		expect(indices).toEqual([0, 1]);
	});

	it('accepts every website color and dither combination through the public package', async () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle({
			id: 1,
			type: 'load-source',
			sourceId: 'modes',
			source: new ImageData(new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255]), 2, 1)
		});
		const colors: ColorSpaceId[] = [
			'srgb',
			'linear-rgb',
			'oklab',
			'cielab',
			'oklch',
			'weighted-rgb',
			'weighted-rgb-601',
			'weighted-rgb-709',
			'cielab-ciede2000',
			'oklch-euclidean',
			'oklch-circular-hue',
			'cielch',
			'cielch-euclidean',
			'cielch-circular-hue',
			'ycbcr'
		];
		const algorithms: DitherId[] = [
			'none',
			'bayer-2',
			'bayer-4',
			'bayer-8',
			'bayer-16',
			'ordered-3x1',
			'ordered-4x1',
			'ordered-4x2',
			'ordered-5x3',
			'random',
			'floyd-steinberg',
			'sierra',
			'sierra-lite',
			'atkinson',
			'jarvis-judice-ninke',
			'stucki',
			'burkes',
			'two-row-sierra',
			'fan',
			'shiau-fan',
			'shiau-fan-2',
			'simple-2d',
			'blue-noise',
			'yliluoma-2',
			'yliluoma-4',
			'yliluoma-8',
			'yliluoma-16'
		];
		for (const colorSpace of colors) {
			for (const algorithm of algorithms) {
				const modeSettings = {
					...settings,
					colorSpace,
					output: { ...settings.output, width: 2, height: 1, crop: undefined },
					dither: { ...settings.dither, algorithm, useColorSpace: true }
				};
				const response = await pipeline.handleAsync(
					{
						id: 2,
						type: 'process',
						sourceId: 'modes',
						settings: modeSettings,
						palette: uploaded.palette.slice(0, 2),
						settingsHash: `${colorSpace}/${algorithm}`
					},
					() => undefined
				);
				if (response?.type !== 'complete') throw new Error('Expected mode output.');
				expect(response.image.indices, `${colorSpace}/${algorithm}`).toEqual(
					new Uint8Array([0, 1])
				);
			}
		}
	});

	it('decodes an upload, packs the crop, processes settings, and renders and exports persisted indices', async () => {
		const blob = encodeIndexedPng(uploaded);
		const decoded = await decodeBlob(blob);
		const sourceRecord = validateSourceImageRecord({
			blob,
			name: 'website-fixture.png',
			width: decoded.width,
			height: decoded.height,
			type: blob.type,
			updatedAt: 0
		});
		expect(sourceRecord.width).toBe(4);
		const sourceBefore = decoded.imageData.data.slice();
		const pipeline = new ProcessorWorkerPipeline();
		await pipeline.handleAsync(
			validateWorkerRequest({
				id: 1,
				type: 'load-source',
				sourceId: 'upload',
				source: decoded.imageData
			}),
			() => undefined
		);
		const palette = [
			...uploaded.palette.slice(0, 3),
			{ ...uploaded.palette[0]!, name: 'Duplicate ink', key: 'duplicate', tags: ['second'] }
		];
		const request = validateWorkerRequest({
			id: 2,
			type: 'process',
			sourceId: 'upload',
			settings,
			palette,
			settingsHash: 'crop-settings'
		});
		const response = validateWorkerResponse(await pipeline.handleAsync(request, () => undefined));
		if (response.type !== 'complete') throw new Error('Expected indexed output.');
		const image = validateProcessedImage(structuredClone(response.image));
		expect(image.indices).toEqual(new Uint8Array([0, 0, 1, 1, 2, 2, 1, 1]));
		expect(image.palette).toEqual(palette);
		expect(image.transparentIndex).toBe(2);
		expect(image.settingsHash).toBe('crop-settings');
		expect(decoded.imageData.data).toEqual(sourceBefore);
		const preview = processedToImageData(image);
		const exported = await decodeBlob(encodeIndexedPng(image));
		expect(exported.imageData.data).toEqual(preview.data);
		expect(Array.from(preview.data.slice(16, 24))).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);

		const matteSettings = { ...settings, output: { ...settings.output, alphaMode: 'matte' } };
		const matteResponse = validateWorkerResponse(
			await pipeline.handleAsync(
				validateWorkerRequest({
					id: 3,
					type: 'process',
					sourceId: 'upload',
					settings: matteSettings,
					palette,
					settingsHash: 'matte-settings'
				}),
				() => undefined
			)
		);
		if (matteResponse.type !== 'complete') throw new Error('Expected matte output.');
		expect(matteResponse.image.indices).toEqual(new Uint8Array([0, 0, 1, 1, 1, 1, 1, 1]));
		expect(image.indices).toEqual(new Uint8Array([0, 0, 1, 1, 2, 2, 1, 1]));
	});

	it('uses the packed crop boundary for filtering and exposes fractional crop refusal', async () => {
		const decoded = await decodeBlob(encodeIndexedPng(uploaded));
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle({ id: 1, type: 'load-source', sourceId: 'upload', source: decoded.imageData });
		const onePixel = {
			...settings,
			output: { ...settings.output, resize: 'lanczos3', crop: { x: 2, y: 0, width: 1, height: 1 } }
		} satisfies ProcessingSettings;
		const response = await pipeline.handleAsync(
			{
				id: 2,
				type: 'process',
				sourceId: 'upload',
				settings: onePixel,
				palette: uploaded.palette,
				settingsHash: 'edge'
			},
			() => undefined
		);
		if (response?.type !== 'complete') throw new Error('Expected cropped output.');
		expect(response.image.indices).toEqual(new Uint8Array(8).fill(1));
		await expect(
			pipeline.handleAsync(
				{
					id: 3,
					type: 'process',
					sourceId: 'upload',
					settings: {
						...onePixel,
						output: { ...onePixel.output, crop: { x: 0.5, y: 0, width: 1, height: 1 } }
					},
					palette: uploaded.palette,
					settingsHash: 'fractional'
				},
				() => undefined
			)
		).rejects.toThrow(/fractional crop/);
	});
});

describe('smooth-gradient adaptive placement', () => {
	// 192x16, identical rows: flat | hard step | flat | 16px ramp | flat | 96px ramp.
	const WIDTH = 192;
	const HEIGHT = 16;
	function rampSource() {
		const data = new Uint8ClampedArray(WIDTH * HEIGHT * 4);
		for (let y = 0; y < HEIGHT; y++) {
			for (let x = 0; x < WIDTH; x++) {
				const offset = (y * WIDTH + x) * 4;
				let r: number;
				let g: number;
				let b: number;
				if (x < 32) {
					r = 20;
					g = 20;
					b = 30;
				} else if (x < 64) {
					r = 0;
					g = 128;
					b = 128;
				} else if (x < 80) {
					r = g = b = 60 + (x - 64) * 6;
				} else if (x < 96) {
					r = g = b = 150;
				} else {
					r = g = b = Math.round(((x - 96) * 255) / 95);
				}
				data[offset] = r;
				data[offset + 1] = g;
				data[offset + 2] = b;
				data[offset + 3] = 255;
			}
		}
		return new ImageData(data, WIDTH, HEIGHT);
	}

	const rampPalette: EnabledPaletteColor[] = [0, 64, 128, 192, 255].map((level) => {
		const key = `#${level.toString(16).padStart(2, '0').repeat(3)}`;
		return {
			name: `Grey ${level}`,
			key,
			rgb: { r: level, g: level, b: level },
			kind: 'custom',
			enabled: true
		};
	});
	rampPalette.push({
		name: 'Teal',
		key: '#008080',
		rgb: { r: 0, g: 128, b: 128 },
		kind: 'custom',
		enabled: true
	});

	const rampOutput = {
		...settings.output,
		width: WIDTH,
		height: HEIGHT,
		crop: undefined
	};
	const rampSettings = (dither: Partial<ProcessingSettings['dither']>, useColorSpace = false) =>
		({
			...settings,
			colorSpace: 'oklab',
			output: rampOutput,
			dither: {
				...settings.dither,
				strength: 30,
				useColorSpace,
				...dither
			}
		}) satisfies ProcessingSettings;

	function region(indices: Uint8Array, x0: number, x1: number) {
		const pixels = new Uint8Array((x1 - x0 + 1) * HEIGHT);
		for (let y = 0; y < HEIGHT; y++) {
			pixels.set(indices.subarray(y * WIDTH + x0, y * WIDTH + x1 + 1), y * (x1 - x0 + 1));
		}
		return pixels;
	}

	let requestId = 1;
	async function run(
		pipeline: ProcessorWorkerPipeline,
		sourceId: string,
		dither: Partial<ProcessingSettings['dither']>,
		useColorSpace = false
	) {
		const response = await pipeline.handleAsync(
			{
				id: ++requestId,
				type: 'process',
				sourceId,
				settings: rampSettings(dither, useColorSpace),
				palette: rampPalette,
				settingsHash: `ramp-${sourceId}-${JSON.stringify(dither)}-${useColorSpace}`
			},
			() => undefined
		);
		if (response?.type !== 'complete') throw new Error('Expected ramp output.');
		return response.image.indices;
	}

	it('Adaptive dithers long smooth ramps only, and a larger minimum excludes a shorter ramp', async () => {
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle({
			id: 1,
			type: 'load-source',
			sourceId: 'ramp',
			source: rampSource()
		});
		const baseline = await run(pipeline, 'ramp', { algorithm: 'none' });

		// x 0..63 covers both flats and the hard step with a one-pixel margin; x 80..95 is flat.
		function expectFlatsUnchanged(indices: Uint8Array) {
			expect(region(indices, 0, 63)).toEqual(region(baseline, 0, 63));
			expect(region(indices, 80, 95)).toEqual(region(baseline, 80, 95));
		}
		function expectLongRampDithered(indices: Uint8Array) {
			expect(region(indices, 96, 191)).not.toEqual(region(baseline, 96, 191));
		}
		function expectShortRampClean(indices: Uint8Array) {
			expect(region(indices, 64, 79)).toEqual(region(baseline, 64, 79));
		}
		function expectShortRampDithered(indices: Uint8Array) {
			expect(region(indices, 64, 79)).not.toEqual(region(baseline, 64, 79));
		}

		for (const algorithm of ['bayer-16', 'floyd-steinberg'] as const) {
			const min24 = await run(pipeline, 'ramp', {
				algorithm,
				placement: 'adaptive',
				placementMinGradientSize: 24
			});
			expectFlatsUnchanged(min24);
			expectLongRampDithered(min24);
			expectShortRampClean(min24);

			const min8 = await run(pipeline, 'ramp', {
				algorithm,
				placement: 'adaptive',
				placementMinGradientSize: 8
			});
			expectFlatsUnchanged(min8);
			expectShortRampDithered(min8);
		}

		const colorSpaceMin24 = await run(
			pipeline,
			'ramp',
			{ algorithm: 'bayer-16', placement: 'adaptive', placementMinGradientSize: 24 },
			true
		);
		expectFlatsUnchanged(colorSpaceMin24);
		expectLongRampDithered(colorSpaceMin24);
		expectShortRampClean(colorSpaceMin24);

		const colorSpaceMin8 = await run(
			pipeline,
			'ramp',
			{ algorithm: 'bayer-16', placement: 'adaptive', placementMinGradientSize: 8 },
			true
		);
		expectFlatsUnchanged(colorSpaceMin8);
		expectShortRampDithered(colorSpaceMin8);
	});
});
