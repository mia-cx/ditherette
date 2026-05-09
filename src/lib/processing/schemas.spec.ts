import { describe, expect, it } from 'vitest';
import {
	assertIndexBuffer,
	assertPaletteForIndexedOutput,
	validateProcessedImage,
	validateSourceImageRecord,
	validateWorkerRequest,
	validateWorkerResponse
} from './schemas';
import type { DitherSettings, EnabledPaletteColor, OutputSettings, ProcessedImage } from './types';

class TestImageData implements ImageData {
	readonly data: Uint8ClampedArray<ArrayBuffer>;
	readonly width: number;
	readonly height: number;
	readonly colorSpace: PredefinedColorSpace = 'srgb';

	constructor(width: number, height: number);
	constructor(data: Uint8ClampedArray<ArrayBuffer>, width: number, height?: number);
	constructor(
		dataOrWidth: Uint8ClampedArray<ArrayBuffer> | number,
		widthOrHeight: number,
		height?: number
	) {
		if (typeof dataOrWidth === 'number') {
			this.width = dataOrWidth;
			this.height = widthOrHeight;
			this.data = new Uint8ClampedArray(this.width * this.height * 4);
			return;
		}
		this.data = dataOrWidth;
		this.width = widthOrHeight;
		this.height = height ?? dataOrWidth.length / 4 / widthOrHeight;
	}
}

Object.defineProperty(globalThis, 'ImageData', { value: TestImageData, configurable: true });

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
	{
		name: 'Transparent',
		key: 'transparent',
		kind: 'transparent',
		enabled: true
	}
];

const output: OutputSettings = {
	width: 2,
	height: 1,
	lockAspect: true,
	resize: 'lanczos3',
	alphaMode: 'preserve',
	alphaThreshold: 0,
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
	seed: 0xc0ffee42,
	useColorSpace: false
};

function processedImage(overrides: Partial<ProcessedImage> = {}): ProcessedImage {
	return {
		width: 2,
		height: 1,
		indices: new Uint8Array([0, 1]),
		palette,
		transparentIndex: 1,
		warnings: [],
		settingsHash: 'hash',
		updatedAt: 1,
		...overrides
	};
}

describe('processing schemas', () => {
	it('accepts valid source image records', () => {
		const record = {
			blob: new Blob(['image'], { type: 'image/png' }),
			name: 'test.png',
			width: 320,
			height: 200,
			type: 'image/png',
			updatedAt: 1
		};

		expect(validateSourceImageRecord(record)).toMatchObject({ width: 320, height: 200 });
	});

	it('rejects malformed source image records from persistence', () => {
		expect(() => validateSourceImageRecord({ blob: 'not a blob' })).toThrow(/blob/i);
	});

	it('accepts valid indexed processed images', () => {
		expect(validateProcessedImage(processedImage())).toMatchObject({ width: 2, height: 1 });
	});

	it('rejects invalid processed dimensions', () => {
		expect(() => validateProcessedImage(processedImage({ width: 0 }))).toThrow(/width/i);
	});

	it('rejects index buffers whose length does not match dimensions', () => {
		expect(() => validateProcessedImage(processedImage({ indices: new Uint8Array([0]) }))).toThrow(
			/dimensions/i
		);
	});

	it('rejects index buffers that reference missing palette entries', () => {
		expect(() => assertIndexBuffer(new Uint8Array([0, 2]), 2, 1, palette.length)).toThrow(
			/palette/i
		);
	});

	it('rejects palettes outside indexed PNG bounds', () => {
		expect(() => assertPaletteForIndexedOutput([])).toThrow(/1–256/);
		expect(() =>
			assertPaletteForIndexedOutput(Array.from({ length: 257 }, () => palette[0]))
		).toThrow(/1–256/);
	});

	it('rejects visible palette colors without RGB channels', () => {
		expect(() =>
			assertPaletteForIndexedOutput([{ name: 'Bad', key: 'bad', kind: 'free', enabled: true }])
		).toThrow(/RGB/i);
	});

	it('rejects invalid transparent indexes', () => {
		expect(() => validateProcessedImage(processedImage({ transparentIndex: 2 }))).toThrow(
			/transparent index/i
		);
	});

	it('deep-validates worker request settings', () => {
		expect(() =>
			validateWorkerRequest({
				id: 1,
				source: new ImageData(2, 1),
				settings: { output: { ...output, width: '2' }, dither, colorSpace: 'oklab' },
				palette,
				settingsHash: 'hash'
			})
		).toThrow(/width/i);
	});

	it('accepts valid worker requests after shaping settings', () => {
		const request = validateWorkerRequest({
			id: 1,
			source: new ImageData(2, 1),
			settings: { output, dither, colorSpace: 'oklab' },
			palette,
			settingsHash: 'hash'
		});

		expect(request.settings.output.width).toBe(2);
		expect(request.settings.colorSpace).toBe('oklab');
	});

	it('rejects malformed worker progress responses', () => {
		expect(() =>
			validateWorkerResponse({ id: 1, type: 'progress', stage: null, progress: NaN })
		).toThrow(/progress/i);
	});

	it('validates worker complete responses through the processed-image schema', () => {
		expect(() =>
			validateWorkerResponse({
				type: 'complete',
				id: 1,
				image: processedImage({ indices: new Uint8Array([3, 0]) })
			})
		).toThrow(/palette/i);
	});
});
