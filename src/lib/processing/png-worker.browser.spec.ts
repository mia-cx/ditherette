import { describe, expect, it } from 'vitest';
import { decodePng, encodeIndexedPng } from './png';
import { encodeIndexedPngOffThread } from './png-worker';
import type { ProcessedImage } from './types';

function image(width: number, height: number): ProcessedImage {
	return {
		width,
		height,
		indices: Uint8Array.from({ length: width * height }, (_, i) => i % 4),
		palette: [
			{ name: 'Dark', key: '#010203', rgb: { r: 1, g: 2, b: 3 }, kind: 'free', enabled: true },
			{
				name: 'Light',
				key: '#fffefd',
				rgb: { r: 255, g: 254, b: 253 },
				kind: 'free',
				enabled: true
			},
			{ name: 'Duplicate', key: '#010203', rgb: { r: 1, g: 2, b: 3 }, kind: 'free', enabled: true },
			{
				name: 'Clear',
				key: 'transparent',
				rgb: { r: 91, g: 12, b: 37 },
				kind: 'transparent',
				enabled: true
			}
		],
		transparentIndex: 3,
		warnings: [],
		settingsHash: 'browser',
		updatedAt: 0
	};
}

describe('real indexed PNG worker', () => {
	it('decodes exact colours, duplicate indices and transparent RGB without detaching the result', async () => {
		const result = image(2, 2);
		const blob = await encodeIndexedPngOffThread(result);
		const png = new Uint8Array(await blob.arrayBuffer());
		expect(png).toEqual(new Uint8Array(await encodeIndexedPng(result).arrayBuffer()));
		expect((await decodePng(png))?.data).toEqual(
			new Uint8ClampedArray([1, 2, 3, 255, 255, 254, 253, 255, 1, 2, 3, 255, 91, 12, 37, 0])
		);
		expect(result.indices).toEqual(new Uint8Array([0, 1, 2, 3]));
	});

	it('keeps animation frames running during a 3 MP export and returns identical PNG bytes', async () => {
		const result = image(2048, 1536);
		let frames = 0;
		let frame: number;
		const tick = () => {
			frames++;
			frame = requestAnimationFrame(tick);
		};
		frame = requestAnimationFrame(tick);
		let blob: Blob;
		try {
			blob = await encodeIndexedPngOffThread(result);
		} finally {
			cancelAnimationFrame(frame);
		}
		expect(frames).toBeGreaterThan(0);
		const digest = async (png: Blob) =>
			Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', await png.arrayBuffer())));
		expect(await digest(blob)).toEqual(await digest(encodeIndexedPng(result)));
		expect(result.indices.byteLength).toBe(2048 * 1536);
		expect(result.indices[2048 * 1536 - 1]).toBe(3);
	});

	it('cancels a real worker before completion', async () => {
		const controller = new AbortController();
		const pending = encodeIndexedPngOffThread(image(4096, 4096), controller.signal);
		controller.abort();
		await expect(pending).rejects.toMatchObject({ name: 'AbortError' });
	});
});
