import { describe, expect, it } from 'vitest';
import {
	appendMetricsSample,
	estimateProcessingMemory,
	percentile,
	summarizeTimingHistory,
	vectorImageBytes,
	type ProcessingMetricsSample
} from './metrics';
import type { ProcessingSettings } from './types';

const settings: ProcessingSettings = {
	output: {
		width: 10,
		height: 10,
		lockAspect: true,
		resize: 'bilinear',
		alphaMode: 'preserve',
		alphaThreshold: 0,
		matteKey: '#FFFFFF',
		autoSizeOnUpload: false,
		scaleFactor: 1
	},
	dither: {
		algorithm: 'none',
		strength: 100,
		placement: 'everywhere',
		placementRadius: 3,
		placementThreshold: 12,
		placementSoftness: 8,
		serpentine: true,
		seed: 1,
		useColorSpace: false
	},
	colorSpace: 'oklab'
};

function sample(overrides: Partial<ProcessingMetricsSample> = {}): ProcessingMetricsSample {
	return {
		id: 1,
		settingsHash: 'hash',
		sourceId: 'source',
		scopeKey: 'source|10x10|bilinear',
		startedAt: 0,
		completedAt: 10,
		totalMs: 10,
		timings: [{ name: 'resize compute', ms: 4 }],
		cache: {
			delta: cacheSnapshot(),
			lifetime: cacheSnapshot()
		},
		memory: estimateProcessingMemory({
			sourceWidth: 10,
			sourceHeight: 10,
			outputWidth: 10,
			outputHeight: 10,
			settings,
			branchCacheBytes: 400,
			branchCacheMaxBytes: 1024
		}),
		outputPixels: 100,
		colorSpace: 'oklab',
		dither: 'none',
		resize: 'bilinear',
		warnings: [],
		...overrides
	};
}

function cacheSnapshot() {
	return {
		sourceLoaded: true,
		sourceBytes: 400,
		branchCount: 1,
		branchBytes: 400,
		branchMaxBytes: 1024,
		resizedHits: 0,
		resizedMisses: 0,
		resizedSets: 0,
		resizedSkips: 0,
		resizedEvictions: 0,
		derivedHits: 0,
		derivedMisses: 0,
		derivedSets: 0,
		derivedSkips: 0,
		derivedEvictions: 0,
		paletteVectorEntries: 0,
		paletteVectorMaxEntries: 16,
		paletteVectorHits: 0,
		paletteVectorMisses: 0,
		paletteVectorSets: 0,
		paletteVectorEvictions: 0
	};
}

describe('processing metrics helpers', () => {
	it('calculates deterministic nearest-rank percentiles', () => {
		expect(percentile([], 0.95)).toBe(0);
		expect(percentile([1], 0.95)).toBe(1);
		expect(percentile([1, 2, 3, 4, 5], 0.95)).toBe(5);
		expect(percentile([1, 2, 3, 4, 5], 0.5)).toBe(3);
	});

	it('summarizes total and named stage timings', () => {
		const summary = summarizeTimingHistory([
			sample({ totalMs: 10, timings: [{ name: 'resize compute', ms: 4 }] }),
			sample({
				totalMs: 30,
				timings: [{ name: 'resize compute', ms: 8, replayed: true }]
			})
		]);

		expect(summary[0]).toMatchObject({ name: 'total', average: 20, p95: 30 });
		expect(summary.find((item) => item.name === 'resize compute')).toMatchObject({
			average: 6,
			p95: 8
		});
	});

	it('resets rolling history when the source or output scope changes', () => {
		const first = sample({ scopeKey: 'source|10x10|bilinear' });
		const second = sample({ scopeKey: 'source|20x20|bilinear' });

		const history = appendMetricsSample(appendMetricsSample([], first), second);

		expect(history).toEqual([second]);
	});

	it('caps rolling history length', () => {
		const history = Array.from({ length: 105 }, (_, index) => sample({ id: index })).reduce(
			(current, next) => appendMetricsSample(current, next),
			[] as ProcessingMetricsSample[]
		);

		expect(history).toHaveLength(100);
		expect(history[0]?.id).toBe(5);
	});

	it('estimates vector and dither work memory', () => {
		expect(vectorImageBytes(10, 10)).toBe(1200);
		expect(
			estimateProcessingMemory({
				sourceWidth: 20,
				sourceHeight: 10,
				outputWidth: 10,
				outputHeight: 10,
				settings: {
					...settings,
					dither: { ...settings.dither, algorithm: 'floyd-steinberg' }
				},
				branchCacheBytes: 400,
				branchCacheMaxBytes: 1024
			})
		).toMatchObject({
			sourceBytes: 800,
			resizedBytes: 400,
			indexBytes: 100,
			vectorBytes: 1200,
			ditherWorkBytes: 1200
		});
	});
});
