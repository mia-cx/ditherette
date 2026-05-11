import type { createPaletteMatcher } from '../color';
import type { QuantizeCaches } from '../quantize-shared';
import type { DitherId, ProcessingSettings, Rgb } from '../types';

export type QuantizeAlgorithmContext = {
	image: ImageData;
	indices: Uint8Array;
	matcher: ReturnType<typeof createPaletteMatcher>;
	settings: ProcessingSettings;
	matte: Rgb;
	transparentIndexValue: number;
	fallbackTransparentIndex: number;
	strength: number;
	paletteCacheKey: string;
	caches?: QuantizeCaches;
};

export type QuantizeAlgorithmFamily = 'direct' | 'ordered' | 'random' | 'error-diffusion';

export type QuantizeAlgorithm = {
	id: DitherId;
	family: QuantizeAlgorithmFamily;
	quantize(context: QuantizeAlgorithmContext): void;
};
