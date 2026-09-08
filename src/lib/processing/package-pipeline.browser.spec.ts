import { afterEach, describe, expect, it, vi } from 'vitest';
import { decodeBlob } from './image-decode';
import { encodeIndexedPng } from './png';
import { processedToImageData } from './render';
import {
	validateProcessedImage,
	validateSourceImageRecord,
	validateWorkerRequest,
	validateWorkerResponse
} from './schemas';
import type { ColorSpaceId, DitherId, ProcessedImage, ProcessingSettings } from './types';
import { ProcessorWorkerPipeline } from './worker-pipeline';

afterEach(() => vi.unstubAllEnvs());

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
		placementRadius: 3,
		placementThreshold: 12,
		placementSoftness: 8,
		serpentine: true,
		seed: 1,
		useColorSpace: false
	},
	colorSpace: 'srgb'
};

describe('installed package website integration', () => {
	it('accepts every website color and dither combination through the public package', async () => {
		vi.stubEnv('DEV', true);
		vi.stubEnv('VITE_DITHERETTE_WASM_PROCESS', 'true');
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{
				id: 1,
				type: 'load-source',
				sourceId: 'modes',
				source: new ImageData(new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255]), 2, 1)
			},
			() => undefined
		);
		const colors: ColorSpaceId[] = [
			'srgb',
			'linear-rgb',
			'oklab',
			'cielab',
			'oklch',
			'weighted-rgb',
			'weighted-rgb-601',
			'weighted-rgb-709'
		];
		const algorithms: DitherId[] = [
			'none',
			'bayer-2',
			'bayer-4',
			'bayer-8',
			'bayer-16',
			'random',
			'floyd-steinberg',
			'sierra',
			'sierra-lite'
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
		vi.stubEnv('DEV', true);
		vi.stubEnv('VITE_DITHERETTE_WASM_PROCESS', 'true');
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
		pipeline.handle(
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
		vi.stubEnv('DEV', true);
		vi.stubEnv('VITE_DITHERETTE_WASM_PROCESS', 'true');
		const decoded = await decodeBlob(encodeIndexedPng(uploaded));
		const pipeline = new ProcessorWorkerPipeline();
		pipeline.handle(
			{ id: 1, type: 'load-source', sourceId: 'upload', source: decoded.imageData },
			() => undefined
		);
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
