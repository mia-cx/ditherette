import type { QuantizeAlgorithm } from './types';

export const randomAlgorithm: QuantizeAlgorithm = {
	id: 'random',
	family: 'random',
	quantize(context) {
		context.runDirect();
	}
};
