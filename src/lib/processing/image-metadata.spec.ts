import { describe, expect, it } from 'vitest';
import { readImageDimensions, validateSourceBlob } from './image-metadata';

function pngHeader(width: number, height: number) {
	const bytes = new Uint8Array(33);
	bytes.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
	bytes[11] = 13;
	bytes[12] = 0x49;
	bytes[13] = 0x48;
	bytes[14] = 0x44;
	bytes[15] = 0x52;
	bytes[16] = (width >>> 24) & 0xff;
	bytes[17] = (width >>> 16) & 0xff;
	bytes[18] = (width >>> 8) & 0xff;
	bytes[19] = width & 0xff;
	bytes[20] = (height >>> 24) & 0xff;
	bytes[21] = (height >>> 16) & 0xff;
	bytes[22] = (height >>> 8) & 0xff;
	bytes[23] = height & 0xff;
	return new Blob([bytes], { type: 'image/png' });
}

function gifHeader(width: number, height: number) {
	const bytes = new Uint8Array(10);
	bytes.set([...new TextEncoder().encode('GIF89a')]);
	bytes[6] = width & 0xff;
	bytes[7] = (width >> 8) & 0xff;
	bytes[8] = height & 0xff;
	bytes[9] = (height >> 8) & 0xff;
	return new Blob([bytes], { type: 'image/gif' });
}

describe('readImageDimensions', () => {
	it('reads PNG dimensions without raster decode', async () => {
		await expect(readImageDimensions(pngHeader(320, 200))).resolves.toEqual({
			width: 320,
			height: 200
		});
	});

	it('reads GIF dimensions without raster decode', async () => {
		await expect(readImageDimensions(gifHeader(64, 48))).resolves.toEqual({
			width: 64,
			height: 48
		});
	});

	it('rejects oversized image headers before decode', async () => {
		await expect(validateSourceBlob(pngHeader(50_000, 50_000))).rejects.toThrow(/too large/i);
	});

	it('rejects PNGs without an IHDR chunk', async () => {
		const bytes = new Uint8Array(await pngHeader(320, 200).arrayBuffer());
		bytes[12] = 0x74;
		await expect(readImageDimensions(new Blob([bytes], { type: 'image/png' }))).rejects.toThrow(
			/IHDR/i
		);
	});
});
