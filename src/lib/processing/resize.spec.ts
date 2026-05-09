import { describe, expect, it } from 'vitest';
import { resizeImageData } from './resize';
import { fitOutputSizeToBounds, validateSourceImageSize } from './types';

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

function solidImage(
	width: number,
	height: number,
	rgba: readonly [number, number, number, number]
) {
	const data = new Uint8ClampedArray(width * height * 4);
	for (let offset = 0; offset < data.length; offset += 4) {
		data[offset] = rgba[0];
		data[offset + 1] = rgba[1];
		data[offset + 2] = rgba[2];
		data[offset + 3] = rgba[3];
	}
	return new ImageData(data, width, height);
}

describe('resizeImageData', () => {
	it('letterboxes contain output instead of smearing clamped edge pixels', () => {
		const output = resizeImageData(solidImage(4, 2, [255, 0, 0, 255]), 4, 4, 'contain', 'nearest');

		expect([...output.data.slice(0, 16)]).toEqual(new Array(16).fill(0));
		expect([...output.data.slice(16, 32)]).toEqual([
			255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255
		]);
		expect([...output.data.slice(48, 64)]).toEqual(new Array(16).fill(0));
	});

	it('does not bleed RGB from transparent pixels during bilinear resize', () => {
		const source = new ImageData(new Uint8ClampedArray([255, 0, 0, 0, 0, 0, 255, 255]), 2, 1);

		const output = resizeImageData(source, 1, 1, 'stretch', 'bilinear');

		expect([...output.data]).toEqual([0, 0, 255, 128]);
	});

	it('allows source dimensions up to the source limit when resizing down', () => {
		const output = resizeImageData(
			solidImage(20_000, 1, [0, 0, 0, 255]),
			1,
			1,
			'stretch',
			'nearest'
		);

		expect(output.width).toBe(1);
		expect(output.height).toBe(1);
	});
});

describe('source and output bounds', () => {
	it('preserves source aspect when fitting oversized output dimensions', () => {
		const fitted = fitOutputSizeToBounds(50_000, 100);

		expect(fitted).toMatchObject({ width: 16_384, height: 32 });
	});

	it('rejects decoded images over the safe source pixel cap', () => {
		expect(() => validateSourceImageSize(16_384, 16_385)).toThrow(/too large/i);
	});
});
