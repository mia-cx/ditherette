import { describe, expect, it } from 'vitest';
import { fitOutputSizeToBounds, validateSourceImageSize } from './types';

describe('source and output bounds', () => {
	it('preserves source aspect when fitting oversized output dimensions', () => {
		const fitted = fitOutputSizeToBounds(50_000, 100);

		expect(fitted).toMatchObject({ width: 16_384, height: 32 });
	});

	it('rejects decoded images over the safe source pixel cap', () => {
		expect(() => validateSourceImageSize(16_384, 16_385)).toThrow(/too large/i);
	});
});
