import type { DitherId } from './types';

export type BayerSize = 2 | 4 | 8 | 16;

const BAYER_SIZES = {
	'bayer-2': 2,
	'bayer-4': 4,
	'bayer-8': 8,
	'bayer-16': 16
} as const satisfies Partial<Record<DitherId, BayerSize>>;

/** The matrix size a Bayer algorithm uses, or undefined for any other algorithm. */
export function bayerSizeForAlgorithm(algorithm: DitherId | string): BayerSize | undefined {
	return Object.hasOwn(BAYER_SIZES, algorithm)
		? BAYER_SIZES[algorithm as keyof typeof BAYER_SIZES]
		: undefined;
}
