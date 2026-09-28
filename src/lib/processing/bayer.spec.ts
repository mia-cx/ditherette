import { describe, expect, it } from 'vitest';
import { bayerSizeForAlgorithm } from './bayer';

describe('Bayer sizes', () => {
	it('maps algorithms to matrix sizes', () => {
		expect(bayerSizeForAlgorithm('bayer-2')).toBe(2);
		expect(bayerSizeForAlgorithm('bayer-4')).toBe(4);
		expect(bayerSizeForAlgorithm('bayer-8')).toBe(8);
		expect(bayerSizeForAlgorithm('bayer-16')).toBe(16);
		expect(bayerSizeForAlgorithm('random')).toBeUndefined();
	});
});
