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

function patternedImage(width: number, height: number) {
	const data = new Uint8ClampedArray(width * height * 4);
	for (let y = 0; y < height; y++) {
		for (let x = 0; x < width; x++) {
			const offset = (y * width + x) * 4;
			data[offset] = (x * 47 + y * 11) & 255;
			data[offset + 1] = (x * 7 + y * 53) & 255;
			data[offset + 2] = (x * y * 19 + 31) & 255;
			data[offset + 3] = (x + y) % 5 === 0 ? 96 : 255;
		}
	}
	return new ImageData(data, width, height);
}

type ReferenceRect = { x: number; y: number; width: number; height: number };

function referenceLanczosResize(
	source: ImageData,
	width: number,
	height: number,
	sourceRect: ReferenceRect = { x: 0, y: 0, width: source.width, height: source.height },
	targetRect: ReferenceRect = { x: 0, y: 0, width, height }
) {
	const output = new ImageData(width, height);
	const targetLeft = Math.max(0, Math.round(targetRect.x));
	const targetTop = Math.max(0, Math.round(targetRect.y));
	const targetRight = Math.min(width, Math.round(targetRect.x + targetRect.width));
	const targetBottom = Math.min(height, Math.round(targetRect.y + targetRect.height));
	const targetWidth = Math.max(1, targetRight - targetLeft);
	const targetHeight = Math.max(1, targetBottom - targetTop);
	const scaleX = sourceRect.width / targetWidth;
	const scaleY = sourceRect.height / targetHeight;
	for (let y = targetTop; y < targetBottom; y++) {
		for (let x = targetLeft; x < targetRight; x++) {
			const sourceX = sourceRect.x + (x - targetLeft + 0.5) * scaleX - 0.5;
			const sourceY = sourceRect.y + (y - targetTop + 0.5) * scaleY - 0.5;
			const pixel = referenceLanczosSample(source, sourceX, sourceY);
			const offset = (y * width + x) * 4;
			output.data[offset] = pixel[0];
			output.data[offset + 1] = pixel[1];
			output.data[offset + 2] = pixel[2];
			output.data[offset + 3] = pixel[3];
		}
	}
	return output;
}

function referenceLanczosSample(source: ImageData, x: number, y: number) {
	const radius = 3;
	const floorX = Math.floor(x);
	const floorY = Math.floor(y);
	let r = 0;
	let g = 0;
	let b = 0;
	let a = 0;
	let total = 0;
	for (let yy = floorY - radius + 1; yy <= floorY + radius; yy++) {
		const wy = referenceLanczos(y - yy, radius);
		for (let xx = floorX - radius + 1; xx <= floorX + radius; xx++) {
			const weight = referenceLanczos(x - xx, radius) * wy;
			if (weight === 0) continue;
			const pixel = referencePixel(source, xx, yy);
			const alpha = pixel[3] / 255;
			r += pixel[0] * alpha * weight;
			g += pixel[1] * alpha * weight;
			b += pixel[2] * alpha * weight;
			a += pixel[3] * weight;
			total += weight;
		}
	}
	if (total === 0) return [0, 0, 0, 0] as const;
	return referenceUnpremultiply(r / total, g / total, b / total, a / total);
}

function referencePixel(source: ImageData, x: number, y: number) {
	const xx = Math.min(source.width - 1, Math.max(0, x));
	const yy = Math.min(source.height - 1, Math.max(0, y));
	const offset = (yy * source.width + xx) * 4;
	return [
		source.data[offset]!,
		source.data[offset + 1]!,
		source.data[offset + 2]!,
		source.data[offset + 3]!
	] as const;
}

function referenceLanczos(value: number, radius: number) {
	const absolute = Math.abs(value);
	if (absolute >= radius) return 0;
	return referenceSinc(absolute) * referenceSinc(absolute / radius);
}

function referenceSinc(value: number) {
	if (value === 0) return 1;
	const x = Math.PI * value;
	return Math.sin(x) / x;
}

function referenceUnpremultiply(r: number, g: number, b: number, a: number) {
	if (a <= 0) return [0, 0, 0, 0] as const;
	const alpha = a / 255;
	return [
		Math.round(Math.max(0, Math.min(255, r / alpha))),
		Math.round(Math.max(0, Math.min(255, g / alpha))),
		Math.round(Math.max(0, Math.min(255, b / alpha))),
		Math.round(Math.max(0, Math.min(255, a)))
	] as const;
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

	it('matches the reference Lanczos sampler before optimizing the hot path', () => {
		const source = patternedImage(8, 5);
		const cases = [
			{
				output: resizeImageData(source, 11, 8, 'stretch', 'lanczos3'),
				reference: referenceLanczosResize(source, 11, 8)
			},
			{
				output: resizeImageData(source, 5, 3, 'stretch', 'lanczos3'),
				reference: referenceLanczosResize(source, 5, 3)
			},
			{
				output: resizeImageData(source, 5, 5, 'contain', 'lanczos3'),
				reference: referenceLanczosResize(
					source,
					5,
					5,
					{ x: 0, y: 0, width: 8, height: 5 },
					{ x: 0, y: 0.9375, width: 5, height: 3.125 }
				)
			},
			{
				output: resizeImageData(source, 5, 5, 'cover', 'lanczos3'),
				reference: referenceLanczosResize(
					source,
					5,
					5,
					{ x: 1.5, y: 0, width: 5, height: 5 },
					{ x: 0, y: 0, width: 5, height: 5 }
				)
			}
		];

		for (const { output, reference } of cases) {
			expect([...output.data]).toEqual([...reference.data]);
		}
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
