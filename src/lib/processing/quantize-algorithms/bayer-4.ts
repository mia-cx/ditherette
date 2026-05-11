import { quantizeDirect } from './direct-runner';
import type { QuantizeAlgorithm } from './types';

export const bayer4Algorithm: QuantizeAlgorithm = {
	id: 'bayer-4',
	family: 'ordered',
	quantize: quantizeDirect
};
