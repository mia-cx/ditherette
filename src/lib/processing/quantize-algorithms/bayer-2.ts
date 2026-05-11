import { quantizeDirect } from './direct-runner';
import type { QuantizeAlgorithm } from './types';

export const bayer2Algorithm: QuantizeAlgorithm = {
	id: 'bayer-2',
	family: 'ordered',
	quantize: quantizeDirect
};
