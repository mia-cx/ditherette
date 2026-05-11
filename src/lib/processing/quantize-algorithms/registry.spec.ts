import { describe, expect, it } from 'vitest';
import type { DitherId } from '../types';
import { QUANTIZE_ALGORITHMS, quantizeAlgorithmFor } from './registry';

const DITHER_IDS: readonly DitherId[] = [
	'none',
	'bayer-2',
	'bayer-4',
	'bayer-8',
	'bayer-16',
	'floyd-steinberg',
	'sierra',
	'sierra-lite',
	'random'
];

describe('quantize algorithm registry', () => {
	it('has one module-backed entry for every dither algorithm', () => {
		expect(QUANTIZE_ALGORITHMS.map((algorithm) => algorithm.id)).toEqual(DITHER_IDS);
		for (const id of DITHER_IDS) expect(quantizeAlgorithmFor(id).id).toBe(id);
	});

	it('classifies algorithms by implementation family', () => {
		expect(quantizeAlgorithmFor('none').family).toBe('direct');
		expect(quantizeAlgorithmFor('bayer-8').family).toBe('ordered');
		expect(quantizeAlgorithmFor('random').family).toBe('random');
		expect(quantizeAlgorithmFor('floyd-steinberg').family).toBe('error-diffusion');
		expect(quantizeAlgorithmFor('sierra').family).toBe('error-diffusion');
		expect(quantizeAlgorithmFor('sierra-lite').family).toBe('error-diffusion');
	});
});
