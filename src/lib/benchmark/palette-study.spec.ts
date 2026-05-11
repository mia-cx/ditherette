import { beforeAll, describe, expect, it } from 'vitest';
import { paletteStudyResultsToCsv, runPaletteStudy } from './palette-study';

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

describe('palette study harness', () => {
	it('runs native direct byte-rgb studies and validates checksums', () => {
		const data = tinyImageData();
		const result = runPaletteStudy({
			iterations: 1,
			warmups: 0,
			studies: ['direct-byte-rgb'],
			variants: ['scan', 'dense-rgb-distance-tables'],
			colorSpaces: ['srgb'],
			sources: [
				{
					id: 'tiny',
					label: 'Tiny',
					imageData: new ImageData(data, 2, 2)
				}
			]
		});

		expect(result.rows).toHaveLength(2);
		expect(result.rows.every((row) => row.matchesBaseline)).toBe(true);
		expect(result.rows[0]!.pixels).toBe(4);
		expect(result.rows[0]!.uniqueKeys).toBe(3);
		expect(result.rows[1]!.cacheHits).toBeGreaterThan(0);
		expect(paletteStudyResultsToCsv(result)).toContain('candidateEvaluations');
	});

	it('runs kernel-only diffusion studies', () => {
		const result = runPaletteStudy({
			iterations: 1,
			warmups: 0,
			studies: ['diffusion-kernel-only'],
			variants: ['generic-kernel', 'unrolled-kernel', 'rolling-row-kernel'],
			dithers: ['sierra'],
			sources: [
				{
					id: 'tiny',
					label: 'Tiny',
					imageData: new ImageData(tinyImageData(), 2, 2)
				}
			]
		});

		expect(result.rows).toHaveLength(3);
		expect(result.rows.every((row) => row.study === 'diffusion-kernel-only')).toBe(true);
		expect(result.rows.every((row) => row.dither === 'sierra')).toBe(true);
		expect(result.rows.every((row) => row.queries === 4)).toBe(true);
	});

	it('runs Bayer threshold-vector cache studies', () => {
		const result = runPaletteStudy({
			iterations: 1,
			warmups: 0,
			studies: ['bayer-threshold-vector'],
			variants: [
				'threshold-scan',
				'threshold-direct-cache',
				'threshold-direct-cache-23',
				'threshold-direct-cache-24',
				'threshold-probe-4',
				'threshold-unique-map',
				'threshold-unique-map-hot'
			],
			colorSpaces: ['weighted-rgb-601'],
			dithers: ['bayer-2'],
			sources: [
				{
					id: 'tiny',
					label: 'Tiny',
					imageData: new ImageData(tinyImageData(), 2, 2)
				}
			]
		});

		expect(result.rows).toHaveLength(7);
		expect(result.rows.every((row) => row.study === 'bayer-threshold-vector')).toBe(true);
		expect(result.rows.every((row) => row.matchesBaseline)).toBe(true);
	});

	it('replays diffusion traces through exact vector matchers', () => {
		const result = runPaletteStudy({
			iterations: 1,
			warmups: 0,
			studies: ['diffusion-trace'],
			variants: ['scan', 'vp-tree', 'ball-tree', 'previous-verify'],
			colorSpaces: ['oklab'],
			dithers: ['floyd-steinberg'],
			sources: [
				{
					id: 'tiny',
					label: 'Tiny',
					imageData: new ImageData(tinyImageData(), 2, 2)
				}
			]
		});

		expect(result.rows).toHaveLength(4);
		expect(result.rows.every((row) => row.study === 'diffusion-trace')).toBe(true);
		expect(result.rows.every((row) => row.matchesBaseline)).toBe(true);
	});
});

function tinyImageData() {
	return new Uint8ClampedArray([0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 255, 0, 0, 0, 255]);
}
