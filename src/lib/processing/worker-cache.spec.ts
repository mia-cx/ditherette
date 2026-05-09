import { describe, expect, it } from 'vitest';
import { PaletteVectorCache, pipelineBranchKey, PipelineBranchCache } from './worker-cache';

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

describe('pipelineBranchKey', () => {
	it('ignores palette, color-space, and dither settings by only using image-derived inputs', () => {
		const key = pipelineBranchKey({
			sourceId: 'source',
			width: 32,
			height: 16,
			resize: 'bilinear'
		});

		expect(key).toBe('source|32x16|bilinear|0,0,full,full|grade:identity');
	});

	it('changes when source, output size, resize mode, crop, or grade changes', () => {
		const base = pipelineBranchKey({ sourceId: 'a', width: 32, height: 16, resize: 'bilinear' });

		expect(
			pipelineBranchKey({ sourceId: 'b', width: 32, height: 16, resize: 'bilinear' })
		).not.toBe(base);
		expect(
			pipelineBranchKey({ sourceId: 'a', width: 33, height: 16, resize: 'bilinear' })
		).not.toBe(base);
		expect(pipelineBranchKey({ sourceId: 'a', width: 32, height: 16, resize: 'nearest' })).not.toBe(
			base
		);
		expect(
			pipelineBranchKey({
				sourceId: 'a',
				width: 32,
				height: 16,
				resize: 'bilinear',
				crop: { x: 1, y: 2, width: 30, height: 14 }
			})
		).not.toBe(base);
		expect(
			pipelineBranchKey({
				sourceId: 'a',
				width: 32,
				height: 16,
				resize: 'bilinear',
				gradeKey: 'curves-v1'
			})
		).not.toBe(base);
	});
});

describe('PipelineBranchCache', () => {
	it('keeps the active branch plus immediate prior branches by recency', () => {
		const cache = new PipelineBranchCache(3, 1024);
		cache.setResized('a', new ImageData(1, 1));
		cache.setResized('b', new ImageData(1, 1));
		cache.setResized('c', new ImageData(1, 1));
		expect(cache.getResized('a')).toBeDefined();

		cache.setResized('d', new ImageData(1, 1));

		expect(cache.getResized('a')).toBeDefined();
		expect(cache.getResized('b')).toBeUndefined();
		expect(cache.getResized('c')).toBeDefined();
		expect(cache.getResized('d')).toBeDefined();
	});

	it('does not store branches that exceed the byte budget', () => {
		const cache = new PipelineBranchCache(3, 8);

		cache.setResized('too-large', new ImageData(2, 2));

		expect(cache.getResized('too-large')).toBeUndefined();
		expect(cache.size).toBe(0);
	});

	it('tracks resize cache hit and miss counters', () => {
		const cache = new PipelineBranchCache(3, 1024);
		cache.setResized('a', new ImageData(1, 1), 12);

		cache.getResized('a');
		cache.getResized('b');

		expect(cache.getResizedTiming('a')).toBe(12);
		expect(cache.snapshotMetrics()).toMatchObject({
			resizedHits: 1,
			resizedMisses: 1,
			resizedSets: 1
		});
	});

	it('stores color mappings under their branch byte budget', () => {
		const cache = new PipelineBranchCache(2, 64);
		cache.setResized('a', new ImageData(1, 1));

		expect(cache.setColorMapping('a', 'oklab', { cached: true }, 24, 34)).toBe(true);
		expect(cache.getColorMapping('a', 'oklab')).toEqual({ cached: true });
		expect(cache.getColorMapping('a', 'missing')).toBeUndefined();
		expect(cache.bytes).toBe(28);
		expect(cache.getColorMappingTiming('a', 'oklab')).toBe(34);
		expect(cache.snapshotMetrics()).toMatchObject({
			derivedHits: 1,
			derivedMisses: 1,
			derivedSets: 1
		});
	});

	it('skips color mappings that would evict their resized branch', () => {
		const cache = new PipelineBranchCache(2, 16);
		cache.setResized('a', new ImageData(1, 1));

		expect(cache.setColorMapping('a', 'too-large', {}, 16)).toBe(false);
		expect(cache.getResized('a')).toBeDefined();
		expect(cache.getColorMapping('a', 'too-large')).toBeUndefined();
	});

	it('evicts derived mappings before evicting resized branches', () => {
		const cache = new PipelineBranchCache(2, 36);
		cache.setResized('area', new ImageData(2, 2));
		expect(cache.setColorMapping('area', 'quantized', { cached: 'area' }, 4)).toBe(true);
		cache.setResized('lanczos3-aa', new ImageData(2, 2));

		expect(cache.setColorMapping('lanczos3-aa', 'quantized', { cached: 'lanczos3-aa' }, 4)).toBe(
			true
		);

		expect(cache.getResized('area')).toBeDefined();
		expect(cache.getResized('lanczos3-aa')).toBeDefined();
		expect(cache.getColorMapping('area', 'quantized')).toBeUndefined();
		expect(cache.getColorMapping('lanczos3-aa', 'quantized')).toEqual({ cached: 'lanczos3-aa' });
		expect(cache.bytes).toBe(36);
	});

	it('clears all active and prior branches together', () => {
		const cache = new PipelineBranchCache(3, 1024);
		cache.setResized('a', new ImageData(1, 1));
		cache.setResized('b', new ImageData(1, 1));

		cache.clear();

		expect(cache.size).toBe(0);
		expect(cache.bytes).toBe(0);
	});
});

describe('PaletteVectorCache', () => {
	it('keeps palette-derived data separate from image branches', () => {
		const cache = new PaletteVectorCache<object>(2);
		const value = { cached: true };

		cache.set('palette|oklab', value);

		expect(cache.get('palette|oklab')).toBe(value);
		expect(cache.size).toBe(1);
	});

	it('evicts least-recently-used entries by entry count', () => {
		const cache = new PaletteVectorCache<object>(2);
		cache.set('a', {});
		cache.set('b', {});
		expect(cache.get('a')).toBeDefined();

		cache.set('c', {});

		expect(cache.get('a')).toBeDefined();
		expect(cache.get('b')).toBeUndefined();
		expect(cache.get('c')).toBeDefined();
		expect(cache.snapshotMetrics()).toMatchObject({ hits: 3, misses: 1, sets: 3, evictions: 1 });
	});
});
