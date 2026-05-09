import { describe, expect, it } from 'vitest';
import { resizeCacheKey, WorkerResizeCache } from './worker-cache';

class TestImageData implements ImageData {
	readonly data: Uint8ClampedArray<ArrayBuffer>;
	readonly width: number;
	readonly height: number;
	readonly colorSpace: PredefinedColorSpace = 'srgb';

	constructor(width: number, height: number) {
		this.width = width;
		this.height = height;
		this.data = new Uint8ClampedArray(width * height * 4);
	}
}

Object.defineProperty(globalThis, 'ImageData', { value: TestImageData, configurable: true });

describe('resizeCacheKey', () => {
	it('ignores palette, color-space, and dither settings by only using resize-affecting inputs', () => {
		const key = resizeCacheKey({ sourceId: 'source', width: 32, height: 16, resize: 'bilinear' });

		expect(key).toBe('source|32x16|bilinear|0,0,full,full');
	});

	it('changes when source, output size, resize mode, or crop changes', () => {
		const base = resizeCacheKey({ sourceId: 'a', width: 32, height: 16, resize: 'bilinear' });

		expect(resizeCacheKey({ sourceId: 'b', width: 32, height: 16, resize: 'bilinear' })).not.toBe(
			base
		);
		expect(resizeCacheKey({ sourceId: 'a', width: 33, height: 16, resize: 'bilinear' })).not.toBe(
			base
		);
		expect(resizeCacheKey({ sourceId: 'a', width: 32, height: 16, resize: 'nearest' })).not.toBe(
			base
		);
		expect(
			resizeCacheKey({
				sourceId: 'a',
				width: 32,
				height: 16,
				resize: 'bilinear',
				crop: { x: 1, y: 2, width: 30, height: 14 }
			})
		).not.toBe(base);
	});
});

describe('WorkerResizeCache', () => {
	it('evicts least-recently-used entries by entry count', () => {
		const cache = new WorkerResizeCache(2, 1024);
		cache.set('a', new ImageData(1, 1));
		cache.set('b', new ImageData(1, 1));
		expect(cache.get('a')).toBeDefined();

		cache.set('c', new ImageData(1, 1));

		expect(cache.get('a')).toBeDefined();
		expect(cache.get('b')).toBeUndefined();
		expect(cache.get('c')).toBeDefined();
	});

	it('does not store entries that exceed the byte budget', () => {
		const cache = new WorkerResizeCache(2, 8);

		cache.set('too-large', new ImageData(2, 2));

		expect(cache.get('too-large')).toBeUndefined();
		expect(cache.size).toBe(0);
	});
});
