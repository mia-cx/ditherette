import { quantizeDirect } from './direct-runner';
import { quantizeErrorDiffusion } from './error-diffusion-runner';
import type { QuantizeAlgorithm, QuantizeAlgorithmContext } from './types';

function quantizeSierraLite(context: QuantizeAlgorithmContext) {
	if (context.strength > 0) {
		quantizeErrorDiffusion(context);
		return;
	}
	quantizeDirect(context);
}

export const sierraLiteAlgorithm: QuantizeAlgorithm = {
	id: 'sierra-lite',
	family: 'error-diffusion',
	quantize: quantizeSierraLite
};
