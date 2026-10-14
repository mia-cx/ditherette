import { describe, expect, it } from 'vitest';
import { inflateSync } from 'node:zlib';
import { decodePng, encodeIndexedPng } from './png';
import type { ProcessedImage } from './types';

function sampleImage(): ProcessedImage {
	return {
		width: 2,
		height: 1,
		indices: new Uint8Array([0, 1]),
		palette: [
			{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
			{ name: 'Transparent', key: 'transparent', kind: 'transparent', enabled: true }
		],
		transparentIndex: 1,
		warnings: [],
		settingsHash: 'test',
		updatedAt: 0
	};
}

async function bytes(blob: Blob) {
	return new Uint8Array(await blob.arrayBuffer());
}

function chunkNames(bytes: Uint8Array) {
	const names: string[] = [];
	let offset = 8;
	while (offset < bytes.length) {
		const length =
			(bytes[offset] << 24) |
			(bytes[offset + 1] << 16) |
			(bytes[offset + 2] << 8) |
			bytes[offset + 3];
		names.push(String.fromCharCode(...bytes.subarray(offset + 4, offset + 8)));
		offset += 12 + length;
	}
	return names;
}

describe('encodeIndexedPng', () => {
	it.each([
		[1, 1],
		[255, 256], // Exactly one stored block, including filter bytes.
		[256, 256], // Block boundary falls inside a row.
		[16384, 9] // Multiple blocks and checksum batches.
	])('decodes exact indices, palette and alpha for %ix%i', async (width, height) => {
		const palette: ProcessedImage['palette'] = Array.from({ length: 256 }, (_, i) => ({
			name: `Colour ${i}`,
			key: `${i}`,
			kind: i === 77 ? 'transparent' : 'free',
			rgb: { r: i, g: 255 - i, b: (i * 37) % 256 },
			enabled: true
		}));
		const image = {
			...sampleImage(),
			width,
			height,
			palette,
			transparentIndex: 77,
			indices: Uint8Array.from({ length: width * height }, (_, i) => (i * 31 + 77) % 256)
		};
		const png = await bytes(encodeIndexedPng(image));
		const chunks = new Map<string, Uint8Array>();
		for (let offset = 8; offset < png.length; ) {
			const length = new DataView(png.buffer).getUint32(offset);
			const name = String.fromCharCode(...png.subarray(offset + 4, offset + 8));
			const end = offset + 8 + length;
			// Independent bitwise CRC verifies chunk framing, including the final IDAT checksum.
			let crc = 0xffffffff;
			for (const byte of png.subarray(offset + 4, end)) {
				crc ^= byte;
				for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
			}
			expect(new DataView(png.buffer).getUint32(end)).toBe((crc ^ 0xffffffff) >>> 0);
			chunks.set(name, png.subarray(offset + 8, end));
			offset = end + 4;
		}
		expect(chunks.get('IHDR')?.[8]).toBe(8);
		expect(chunks.get('IHDR')?.[9]).toBe(3);
		expect(chunks.get('PLTE')).toEqual(
			Uint8Array.from(palette.flatMap((color) => Object.values(color.rgb!)))
		);
		expect(chunks.get('tRNS')).toEqual(Uint8Array.from(palette, (_, i) => (i === 77 ? 0 : 255)));
		const rows = inflateSync(chunks.get('IDAT')!);
		for (let y = 0; y < height; y++) {
			const offset = y * (width + 1);
			expect(rows[offset]).toBe(0);
			expect(new Uint8Array(rows.subarray(offset + 1, offset + 1 + width))).toEqual(
				image.indices.subarray(y * width, (y + 1) * width)
			);
		}
		const decoded = await decodePng(png);
		const expected = Uint8ClampedArray.from(
			Array.from(image.indices).flatMap((i) => [
				palette[i].rgb!.r,
				palette[i].rgb!.g,
				palette[i].rgb!.b,
				i === 77 ? 0 : 255
			])
		);
		expect(decoded?.data).toEqual(expected);
	});
	it('writes an indexed PNG with PLTE and tRNS chunks', async () => {
		const png = await bytes(encodeIndexedPng(sampleImage()));

		expect([...png.slice(0, 8)]).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
		expect(chunkNames(png)).toEqual(['IHDR', 'PLTE', 'tRNS', 'IDAT', 'IEND']);
		expect(png.includes(3)).toBe(true);
	});

	it('is deterministic for identical indexed image data', async () => {
		const first = await bytes(encodeIndexedPng(sampleImage()));
		const second = await bytes(encodeIndexedPng(sampleImage()));

		expect([...first]).toEqual([...second]);
	});

	it('rejects index buffers that do not match dimensions', () => {
		expect(() => encodeIndexedPng({ ...sampleImage(), indices: new Uint8Array([0]) })).toThrow(
			/dimensions/i
		);
	});

	it('rejects indices outside the palette', () => {
		expect(() => encodeIndexedPng({ ...sampleImage(), indices: new Uint8Array([0, 2]) })).toThrow(
			/palette/i
		);
	});
});
