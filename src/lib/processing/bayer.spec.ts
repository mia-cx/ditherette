import { describe, expect, it } from 'vitest';
import { BAYER_MATRICES, bayerSizeForAlgorithm, flattenedBayerMatrix } from './bayer';

describe('Bayer matrices', () => {
	it('uses explicit ordered matrices', () => {
		expect(BAYER_MATRICES[2]).toEqual([
			[0, 2],
			[3, 1]
		]);
		expect(BAYER_MATRICES[4]).toEqual([
			[0, 8, 2, 10],
			[12, 4, 14, 6],
			[3, 11, 1, 9],
			[15, 7, 13, 5]
		]);
		expect(BAYER_MATRICES[8][0]).toEqual([0, 32, 8, 40, 2, 34, 10, 42]);
		expect(BAYER_MATRICES[16][15]).toEqual([
			255, 127, 223, 95, 247, 119, 215, 87, 253, 125, 221, 93, 245, 117, 213, 85
		]);
	});

	it('maps algorithms to matrix sizes', () => {
		expect(bayerSizeForAlgorithm('bayer-2')).toBe(2);
		expect(bayerSizeForAlgorithm('bayer-4')).toBe(4);
		expect(bayerSizeForAlgorithm('bayer-8')).toBe(8);
		expect(bayerSizeForAlgorithm('bayer-16')).toBe(16);
		expect(bayerSizeForAlgorithm('random')).toBeUndefined();
	});

	it('flattens matrices row-major', () => {
		expect(flattenedBayerMatrix(2)).toEqual([0, 2, 3, 1]);
	});
});
