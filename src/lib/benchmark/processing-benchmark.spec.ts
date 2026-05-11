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

	it('stops after color-space conversion when requested', () => {
		const result = runProcessingBenchmarks({
			matrixDimensions: ['colorSpace'],
			caseIds: ['scale-5-lanczos3-none-oklab'],
			iterations: 1,
			warmups: 0,
			stopAfterStage: 'colorSpaceConvert',
			includePng: false,
			sources: [
				{
					id: 'fixture',
					label: 'Fixture',
					kind: 'synthetic',
					imageData: new ImageData(16, 16)
				}
			]
		});
		const entry = result.results[0]!;

		expect(entry.stages.quantize.meanMs).toBeGreaterThanOrEqual(0);
		expect(
			entry.quantizeSubstages['color space convert composited image']?.meanMs
		).toBeGreaterThanOrEqual(0);
		expect(entry.quantizeSubstages['quantize direct dither+match loop']).toBeUndefined();
		expect(entry.stages.previewRender.meanMs).toBe(0);
	});

	it('can run native-size dither/color-space permutations without resize work', () => {
		const result = runProcessingBenchmarks({
			matrixDimensions: ['dither', 'colorSpace'],
			iterations: 1,
			warmups: 0,
			stopAfterStage: 'quantize',
			includePng: false,
			noResize: true,
			sources: [
				{
					id: 'fixture',
					label: 'Fixture',
					kind: 'synthetic',
					imageData: new ImageData(16, 12)
				}
			]
		});
		const entry = result.results[0]!;

		expect(result.results).toHaveLength(9 * 8);
		expect(entry.case.id).toMatch(/^native-/);
		expect(entry.case.noResize).toBe(true);
		expect(entry.memory.outputPixels).toBe(16 * 12);
		expect(entry.memory.resizedRgbaBytes).toBe(0);
		expect(result.results.every((caseResult) => caseResult.stages.resize.meanMs === 0)).toBe(true);
		expect(
			result.results.every((caseResult) => caseResult.runs.every((run) => !run.resizeCacheHit))
		).toBe(true);
	});

	it('memoizes resize work when benchmarking downstream stages', () => {
		const result = runProcessingBenchmarks({
			matrixDimensions: ['colorSpace'],
			iterations: 1,
			warmups: 1,
			stopAfterStage: 'quantize',
			includePng: false,
			sources: [
				{
					id: 'fixture',
					label: 'Fixture',
					kind: 'synthetic',
					imageData: new ImageData(16, 16)
				}
			]
		});

		expect(result.results).toHaveLength(8);
		expect(result.results.every((entry) => entry.runs.every((run) => run.resizeCacheHit))).toBe(
			true
		);
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
		expect(
			entry.quantizeSubstages['quantize direct dither+match loop']?.meanMs
		).toBeGreaterThanOrEqual(0);
		expect(entry.quantizeCounters['nearest rgb']?.meanMs).toBeGreaterThanOrEqual(0);
		expect(entry.memory.outputPixels).toBe(160 * 120);
		expect(entry.hotspot).toBeTypeOf('string');
		expect(entry.quantizeHotspot).toBeTypeOf('string');
	});

	it('formats machine-readable CSV and a concise table', () => {
		const result = runProcessingBenchmarks({
			profile: 'smoke',
			caseIds: ['smoke-direct'],
			iterations: 1,
			warmups: 0,
			includePng: false
		});

		const csv = benchmarkResultsToCsv(result);
		expect(csv).toContain('caseId');
		expect(csv).toContain('quantize:quantize direct dither+match loop:meanMs');
		expect(csv).toContain('quantize-count:nearest rgb:mean');
		expect(formatBenchmarkTable(result)).toContain('q hotspot');
	});
});
