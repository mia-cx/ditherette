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

	it('routes direct algorithms through the direct runner', () => {
		let directRuns = 0;
		let diffusionRuns = 0;

		quantizeAlgorithmFor('bayer-8').quantize({
			settings: undefined as never,
			strength: 1,
			runDirect: () => directRuns++,
			runErrorDiffusion: () => diffusionRuns++
		});

		expect(directRuns).toBe(1);
		expect(diffusionRuns).toBe(0);
	});

	it('falls back to direct quantization when diffusion strength is zero', () => {
		let directRuns = 0;
		let diffusionRuns = 0;

		quantizeAlgorithmFor('floyd-steinberg').quantize({
			settings: undefined as never,
			strength: 0,
			runDirect: () => directRuns++,
			runErrorDiffusion: () => diffusionRuns++
		});

		expect(directRuns).toBe(1);
		expect(diffusionRuns).toBe(0);
	});
});
