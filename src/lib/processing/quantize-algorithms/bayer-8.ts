import { quantizeDirect } from './direct-runner';
import type { QuantizeAlgorithm } from './types';

export const bayer8Algorithm: QuantizeAlgorithm = {
	id: 'bayer-8',
	family: 'ordered',
	quantize: quantizeDirect
};
