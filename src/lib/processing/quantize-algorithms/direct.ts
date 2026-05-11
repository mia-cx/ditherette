import type { QuantizeAlgorithm } from './types';

export const noneAlgorithm: QuantizeAlgorithm = {
	id: 'none',
	family: 'direct',
	quantize(context) {
		context.runDirect();
	}
};
