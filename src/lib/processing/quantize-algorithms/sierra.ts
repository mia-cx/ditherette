import { quantizeDirect } from './direct-runner';
import { quantizeErrorDiffusion } from './error-diffusion-runner';
import type { QuantizeAlgorithm, QuantizeAlgorithmContext } from './types';

function quantizeSierra(context: QuantizeAlgorithmContext) {
	if (context.strength > 0) {
		quantizeErrorDiffusion(context);
		return;
	}
	quantizeDirect(context);
}

export const sierraAlgorithm: QuantizeAlgorithm = {
	id: 'sierra',
	family: 'error-diffusion',
	quantize: quantizeSierra
};
