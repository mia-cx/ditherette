import { quantizeDirect } from './direct-runner';
import type { QuantizeAlgorithm } from './types';

export const randomAlgorithm: QuantizeAlgorithm = {
	id: 'random',
	family: 'random',
	quantize: quantizeDirect
};
