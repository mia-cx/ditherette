import { afterEach, describe, expect, it, vi } from 'vitest';
import { decodeBlob, NOISY_READBACK_WARNING } from './image-decode';
import { decodePng } from './png';

// Written by a small Python script with zlib, independent of both decoders under test.
const PNG_FIXTURES = {
	'RGBA with every filter type and colour under zero alpha': {
		png: 'iVBORw0KGgoAAAANSUhEUgAAAAQAAAAFCAYAAABirU3bAAAAVUlEQVR42mNgZz7x3ybjKUPhWSbGZUbyDYy3p9v8M02V9QfiTUDMyHRlSgkzEBsD8X8g9mXWWXXmP2vtScfWvyebWms9I1lAMpxTZH05gVqu/JRlBADVBiSKMn963AAAAABJRU5ErkJggg==',
		pixels: [
			7, 3, 200, 255, 60, 104, 229, 0, 113, 205, 2, 1, 166, 50, 31, 128, 219, 151, 60, 254, 16, 252,
			89, 77, 69, 97, 118, 255, 122, 198, 147, 0, 175, 43, 176, 1, 228, 144, 205, 128, 25, 245, 234,
			254, 78, 90, 7, 77, 131, 191, 36, 255, 184, 36, 65, 0, 237, 137, 94, 1, 34, 238, 123, 128, 87,
			83, 152, 254, 140, 184, 181, 77, 193, 29, 210, 255, 246, 130, 239, 0
		]
	},
	'2-bit indexed with partial tRNS': {
		png: 'iVBORw0KGgoAAAANSUhEUgAAAAUAAAACAgMAAADtBP7OAAAADFBMVEX6ChQFBgcAgP9jxyHzRqh+AAAAAnRSTlMAgJsrThgAAAAOSURBVHjaY5BmYHwSAwACtAFdKlWZFAAAAABJRU5ErkJggg==',
		pixels: [
			250, 10, 20, 0, 5, 6, 7, 128, 0, 128, 255, 255, 99, 199, 33, 255, 250, 10, 20, 0, 99, 199, 33,
			255, 0, 128, 255, 255, 5, 6, 7, 128, 250, 10, 20, 0, 5, 6, 7, 128
		]
	},
	'16-bit greyscale with alpha': {
		png: 'iVBORw0KGgoAAAANSUhEUgAAAAIAAAACEAQAAACILxnsAAAAG0lEQVR42mMQ+v///+r/DQws7xoZGUMY6v8DAFatCIHFGvD4AAAAAElFTkSuQmCC',
		// Like browsers, keeps the high byte of each 16-bit sample.
		pixels: [18, 18, 18, 255, 171, 171, 171, 128, 0, 0, 0, 0, 255, 255, 255, 127]
	},
	'RGB with a tRNS colour key': {
		png: 'iVBORw0KGgoAAAANSUhEUgAAAAMAAAABCAIAAACUgoPjAAAABnRSTlMACgAUAB7FNin/AAAAEklEQVR42mPgEpHjEpE/kWIEAAZEAdgyOd7xAAAAAElFTkSuQmCC',
		pixels: [10, 20, 30, 0, 10, 20, 31, 255, 200, 100, 50, 255]
	},
	'4-bit greyscale with a tRNS key': {
		png: 'iVBORw0KGgoAAAANSUhEUgAAAAMAAAABBAAAAAD7e6ZpAAAAAnRSTlMABQb5ObcAAAALSURBVHjaY2D9AAAA/QD2OZVjHQAAAABJRU5ErkJggg==',
		pixels: [0, 0, 0, 255, 85, 85, 85, 0, 255, 255, 255, 255]
	}
};

function pngBytes(base64: string) {
	return Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
}

function spyOnCanvasReadback() {
	return [
		vi.spyOn(CanvasRenderingContext2D.prototype, 'getImageData'),
		vi.spyOn(OffscreenCanvasRenderingContext2D.prototype, 'getImageData')
	];
}

/** A 6x4 JPEG of six 2x2 colour blocks, optionally tagged with an EXIF orientation. */
async function blockJpeg(orientation?: number) {
	const canvas = new OffscreenCanvas(6, 4);
	const context = canvas.getContext('2d')!;
	['#ff0000', '#00ff00', '#0000ff', '#ffff00', '#00ffff', '#ff00ff'].forEach((color, block) => {
		context.fillStyle = color;
		context.fillRect((block % 3) * 2, Math.floor(block / 3) * 2, 2, 2);
	});
	const jpeg = new Uint8Array(
		await (await canvas.convertToBlob({ type: 'image/jpeg', quality: 1 })).arrayBuffer()
	);
	if (orientation === undefined) return new Blob([jpeg], { type: 'image/jpeg' });
	// Big-endian TIFF with one IFD entry: Orientation (0x0112), SHORT, count 1.
	const exif = [
		...[0x45, 0x78, 0x69, 0x66, 0, 0],
		...[0x4d, 0x4d, 0, 0x2a, 0, 0, 0, 8, 0, 1],
		...[0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, orientation, 0, 0],
		...[0, 0, 0, 0]
	];
	const length = exif.length + 2;
	const app1 = [0xff, 0xe1, length >> 8, length & 0xff, ...exif];
	return new Blob([jpeg.subarray(0, 2), new Uint8Array(app1), jpeg.subarray(2)], {
		type: 'image/jpeg'
	});
}

async function canvasDecode(blob: Blob) {
	const bitmap = await createImageBitmap(blob);
	const context = new OffscreenCanvas(bitmap.width, bitmap.height).getContext('2d')!;
	context.drawImage(bitmap, 0, 0);
	return context.getImageData(0, 0, bitmap.width, bitmap.height);
}

afterEach(() => {
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

describe('decodeBlob', () => {
	it.each(Object.entries(PNG_FIXTURES))(
		'decodes a %s PNG byte for byte, with or without ImageDecoder',
		async (_name, fixture) => {
			const readback = spyOnCanvasReadback();
			const blob = new Blob([pngBytes(fixture.png)], { type: 'image/png' });

			const decoded = await decodeBlob(blob);
			vi.stubGlobal('ImageDecoder', undefined);
			const fallback = await decodeBlob(blob);

			expect(Array.from(decoded.imageData.data)).toEqual(fixture.pixels);
			expect(Array.from(fallback.imageData.data)).toEqual(fixture.pixels);
			expect(decoded.warning).toBeUndefined();
			for (const spy of readback) expect(spy).not.toHaveBeenCalled();
		}
	);

	it('leaves interlaced and colour-managed PNGs to the browser', async () => {
		const bytes = pngBytes(
			PNG_FIXTURES['RGBA with every filter type and colour under zero alpha'].png
		);
		const interlaced = bytes.slice();
		interlaced[28] = 1;
		const ihdrEnd = 33;
		const gamma = new Uint8Array([0, 0, 0, 4, ...'gAMA'.split('').map((c) => c.charCodeAt(0))]);
		const withGamma = new Uint8Array([
			...bytes.subarray(0, ihdrEnd),
			...gamma,
			...[0, 0, 0xb1, 0x8f, 0, 0, 0, 0],
			...bytes.subarray(ihdrEnd)
		]);

		expect(await decodePng(interlaced)).toBeUndefined();
		expect(await decodePng(withGamma)).toBeUndefined();
	});

	it.each([1, 2, 3, 4, 5, 6, 7, 8])(
		'applies EXIF orientation %i like createImageBitmap',
		async (orientation) => {
			const blob = await blockJpeg(orientation);
			const expected = await canvasDecode(blob);

			const decoded = await decodeBlob(blob);

			expect([decoded.width, decoded.height]).toEqual([expected.width, expected.height]);
			const worst = decoded.imageData.data.reduce(
				(max, value, index) => Math.max(max, Math.abs(value - expected.data[index]!)),
				0
			);
			expect(worst).toBeLessThanOrEqual(8);
		}
	);

	it('warns only when canvas readback is noisy', async () => {
		vi.stubGlobal('ImageDecoder', undefined);
		const blob = await blockJpeg();

		const clean = await decodeBlob(blob);
		const read = OffscreenCanvasRenderingContext2D.prototype.getImageData;
		vi.spyOn(OffscreenCanvasRenderingContext2D.prototype, 'getImageData').mockImplementation(
			function (this: OffscreenCanvasRenderingContext2D, ...args) {
				const image = read.apply(this, args);
				image.data[0] ^= 1;
				return image;
			}
		);
		const noisy = await decodeBlob(blob);

		expect(clean.warning).toBeUndefined();
		expect(noisy.warning).toBe(NOISY_READBACK_WARNING);
	});
});
