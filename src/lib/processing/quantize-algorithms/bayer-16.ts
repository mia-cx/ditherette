import type { QuantizeAlgorithm } from './types';

export const bayer16Algorithm: QuantizeAlgorithm = {
	id: 'bayer-16',
	family: 'ordered',
	quantize(context) {
		context.runDirect();
	}
};
