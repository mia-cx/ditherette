import type { DitherId } from '../types';
import { bayer16Algorithm } from './bayer-16';
import { bayer2Algorithm } from './bayer-2';
import { bayer4Algorithm } from './bayer-4';
import { bayer8Algorithm } from './bayer-8';
import { noneAlgorithm } from './direct';
import { floydSteinbergAlgorithm, sierraAlgorithm, sierraLiteAlgorithm } from './error-diffusion';
import { randomAlgorithm } from './random';
import type { QuantizeAlgorithm } from './types';

export const QUANTIZE_ALGORITHMS = [
	noneAlgorithm,
	bayer2Algorithm,
	bayer4Algorithm,
	bayer8Algorithm,
	bayer16Algorithm,
	floydSteinbergAlgorithm,
	sierraAlgorithm,
	sierraLiteAlgorithm,
	randomAlgorithm
] as const satisfies readonly QuantizeAlgorithm[];

const ALGORITHMS_BY_ID = new Map<DitherId, QuantizeAlgorithm>(
	QUANTIZE_ALGORITHMS.map((algorithm) => [algorithm.id, algorithm])
);

export function quantizeAlgorithmFor(id: DitherId): QuantizeAlgorithm {
	const algorithm = ALGORITHMS_BY_ID.get(id);
	if (!algorithm) throw new Error(`Unsupported dither algorithm: ${id}`);
	return algorithm;
}
