import { describe, expect, it } from 'vitest';
import { quantizeImage } from './quantize';
import { runRowQuantizeJob } from './quantize-row.worker';
import type {
	ColorSpaceId,
	DitherSettings,
	EnabledPaletteColor,
	OutputSettings,
	ProcessingSettings
} from './types';

class TestImageData implements ImageData {
	readonly data: Uint8ClampedArray<ArrayBuffer>;
	readonly width: number;
	readonly height: number;
	readonly colorSpace: PredefinedColorSpace = 'srgb';

	constructor(data: Uint8ClampedArray<ArrayBuffer>, width: number, height: number) {
		this.data = data;
		this.width = width;
		this.height = height;
	}
}

Object.defineProperty(globalThis, 'ImageData', { value: TestImageData, configurable: true });

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
	{ name: 'Red', key: '#FF0000', rgb: { r: 255, g: 0, b: 0 }, kind: 'free', enabled: true },
	{ name: 'Green', key: '#00FF00', rgb: { r: 0, g: 255, b: 0 }, kind: 'free', enabled: true },
	{ name: 'Blue', key: '#0000FF', rgb: { r: 0, g: 0, b: 255 }, kind: 'free', enabled: true },
	{ name: 'White', key: '#FFFFFF', rgb: { r: 255, g: 255, b: 255 }, kind: 'free', enabled: true },
	{ name: 'Transparent', key: 'transparent', kind: 'transparent', enabled: true }
];

const output: OutputSettings = {
	width: 5,
	height: 3,
	lockAspect: true,
	resize: 'nearest',
	alphaMode: 'preserve',
	alphaThreshold: 10,
	matteKey: '#FFFFFF',
	autoSizeOnUpload: false,
	scaleFactor: 1
};

const dither: DitherSettings = {
	algorithm: 'none',
	strength: 100,
	placement: 'everywhere',
	placementRadius: 3,
	placementThreshold: 12,
	placementSoftness: 8,
	serpentine: true,
	seed: 1,
	useColorSpace: false
};

function image() {
	return new ImageData(
		new Uint8ClampedArray([
			0, 0, 0, 255, 40, 20, 10, 255, 220, 30, 40, 255, 20, 230, 40, 255, 20, 40, 230, 255, 250, 250,
			250, 255, 128, 90, 40, 255, 90, 128, 40, 255, 40, 90, 128, 255, 200, 120, 80, 255, 10, 20, 30,
			0, 240, 10, 200, 255, 10, 240, 200, 255, 200, 10, 240, 255, 120, 120, 120, 255
		]),
		5,
		3
	);
}

function settings(
	algorithm: DitherSettings['algorithm'],
	colorSpace: ColorSpaceId,
	useColorSpace: boolean
): ProcessingSettings {
	return {
		output,
		dither: { ...dither, algorithm, useColorSpace },
		colorSpace
	};
}

describe('row quantize worker job', () => {
	it.each([
		['none', 'srgb', false, 0],
		['none', 'oklab', true, 0],
		['bayer-2', 'oklab', true, 0],
		['bayer-2', 'weighted-rgb-601', true, 0],
		['random', 'srgb', false, 0],
		['random', 'oklab', true, 0]
	] as const)(
		'matches sync quantize output for %s in %s',
		(algorithm, colorSpace, useColorSpace, randomStepStart) => {
			const source = image();
			const nextSettings = settings(algorithm, colorSpace, useColorSpace);
			const expected = quantizeImage(source, palette, nextSettings);

			const actual = runRowQuantizeJob({
				id: 1,
				width: source.width,
				height: source.height,
				startY: 0,
				endY: source.height,
				settings: nextSettings,
				palette,
				sourceRows: source.data,
				randomStepStart
			});

			expect(actual.type).toBe('done');
			expect([...(actual.indices ?? [])]).toEqual([...expected.indices]);
		}
	);

	it('continues random dither sequence for split row jobs', () => {
		const source = image();
		const nextSettings = settings('random', 'srgb', false);
		const expected = quantizeImage(source, palette, nextSettings);
		const randomStepStart = source.width;

		const actual = runRowQuantizeJob({
			id: 2,
			width: source.width,
			height: source.height,
			startY: 1,
			endY: source.height,
			settings: nextSettings,
			palette,
			sourceRows: source.data.slice(source.width * 4),
			randomStepStart
		});

		expect([...(actual.indices ?? [])]).toEqual([...expected.indices.slice(source.width)]);
	});
});
