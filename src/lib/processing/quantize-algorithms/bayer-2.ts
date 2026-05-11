import type { QuantizeAlgorithm } from './types';

export const bayer2Algorithm: QuantizeAlgorithm = {
	id: 'bayer-2',
	family: 'ordered',
	quantize(context) {
		context.runDirect();
	}
};
