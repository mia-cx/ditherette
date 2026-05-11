import { quantizeDirect } from './direct-runner';
import type { QuantizeAlgorithm } from './types';

export const noneAlgorithm: QuantizeAlgorithm = {
	id: 'none',
	family: 'direct',
	quantize: quantizeDirect
};
