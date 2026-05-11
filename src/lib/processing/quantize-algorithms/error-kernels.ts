import type { DitherId } from '../types';

export const ERROR_KERNELS = {
	'floyd-steinberg': [
		[1, 0, 7 / 16],
		[-1, 1, 3 / 16],
		[0, 1, 5 / 16],
		[1, 1, 1 / 16]
	],
	sierra: [
		[1, 0, 5 / 32],
		[2, 0, 3 / 32],
		[-2, 1, 2 / 32],
		[-1, 1, 4 / 32],
		[0, 1, 5 / 32],
		[1, 1, 4 / 32],
		[2, 1, 2 / 32],
		[-1, 2, 2 / 32],
		[0, 2, 3 / 32],
		[1, 2, 2 / 32]
	],
	'sierra-lite': [
		[1, 0, 2 / 4],
		[-1, 1, 1 / 4],
		[0, 1, 1 / 4]
	]
} as const satisfies Partial<Record<DitherId, readonly (readonly [number, number, number])[]>>;

type ErrorDiffusionAlgorithm = keyof typeof ERROR_KERNELS;

export function errorKernelForAlgorithm(algorithm: DitherId) {
	return Object.hasOwn(ERROR_KERNELS, algorithm)
		? ERROR_KERNELS[algorithm as ErrorDiffusionAlgorithm]
		: undefined;
}
