import { describe, expect, it } from 'vitest';
import { encodeIndexedPng } from './png';
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
