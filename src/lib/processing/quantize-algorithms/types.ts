import type { DitherId, ProcessingSettings } from '../types';

export type QuantizeAlgorithmContext = {
	settings: ProcessingSettings;
	strength: number;
	runDirect(): void;
	runErrorDiffusion(): void;
};

export type QuantizeAlgorithmFamily = 'direct' | 'ordered' | 'random' | 'error-diffusion';

export type QuantizeAlgorithm = {
	id: DitherId;
	family: QuantizeAlgorithmFamily;
	quantize(context: QuantizeAlgorithmContext): void;
};
