import type { QuantizeAlgorithm } from './types';

export const bayer8Algorithm: QuantizeAlgorithm = {
	id: 'bayer-8',
	family: 'ordered',
	quantize(context) {
		context.runDirect();
	}
};
