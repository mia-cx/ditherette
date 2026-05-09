import { beforeAll, describe, expect, it } from 'vitest';
import {
	benchmarkCases,
	benchmarkResultsToCsv,
	formatBenchmarkTable,
	runProcessingBenchmarks
} from './processing-benchmark';

beforeAll(() => {
	if ('ImageData' in globalThis) return;
	globalThis.ImageData = class ImageData {
		data: Uint8ClampedArray;
		width: number;
		height: number;

		constructor(dataOrWidth: Uint8ClampedArray | number, width: number, height?: number) {
			if (typeof dataOrWidth === 'number') {
				this.width = dataOrWidth;
				this.height = width;
				this.data = new Uint8ClampedArray(this.width * this.height * 4);
				return;
			}
			this.data = dataOrWidth;
			this.width = width;
			this.height = height ?? Math.floor(dataOrWidth.length / width / 4);
		}
	} as typeof ImageData;
});

describe('processing benchmark harness', () => {
	it('generates exhaustive scale/color-space/dither permutations', () => {
		const cases = benchmarkCases('exhaustive');

		expect(cases).toHaveLength(4 * 7 * 8 * 9);
		expect(cases).toContainEqual(
			expect.objectContaining({
				outputScale: 0.125,
				resize: 'nearest',
				dither: 'none',
				colorSpace: 'oklab'
			})
		);
		expect(cases).toContainEqual(
			expect.objectContaining({
				outputScale: 0.75,
				resize: 'area',
				dither: 'random',
				colorSpace: 'oklch'
			})
		);
	});

	it('can vary only scale and resize and stop after resize', () => {
		const result = runProcessingBenchmarks({
			matrixDimensions: ['scale', 'resize'],
			iterations: 1,
			warmups: 0,
			stopAfterStage: 'resize',
			includePng: false,
			sources: [
				{
					id: 'tiny',
					label: 'Tiny fixture',
					kind: 'synthetic',
					imageData: new ImageData(16, 16)
				}
			]
		});
		const entry = result.results[0]!;

		expect(result.results).toHaveLength(4 * 7);
		expect(entry.stages.resize.meanMs).toBeGreaterThanOrEqual(0);
		expect(entry.stages.quantize.meanMs).toBe(0);
		expect(entry.stages.previewRender.meanMs).toBe(0);
		expect(entry.stages.pngEncode.meanMs).toBe(0);
	});

	it('runs a smoke benchmark and reports stage timings', () => {
		const result = runProcessingBenchmarks({
			profile: 'smoke',
			caseIds: ['smoke-direct'],
			iterations: 1,
			warmups: 0,
			includePng: false
		});
		const entry = result.results[0]!;

		expect(result.version).toBe(1);
		expect(entry.case.id).toBe('smoke-direct');
		expect(entry.stages.resize.meanMs).toBeGreaterThanOrEqual(0);
		expect(entry.stages.quantize.meanMs).toBeGreaterThanOrEqual(0);
		expect(entry.memory.outputPixels).toBe(160 * 120);
		expect(entry.hotspot).toBeTypeOf('string');
	});

	it('formats machine-readable CSV and a concise table', () => {
		const result = runProcessingBenchmarks({
			profile: 'smoke',
			caseIds: ['smoke-direct'],
			iterations: 1,
			warmups: 0,
			includePng: false
		});

		expect(benchmarkResultsToCsv(result)).toContain('caseId');
		expect(formatBenchmarkTable(result)).toContain('hotspot');
	});
});
