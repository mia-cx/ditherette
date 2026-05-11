import { quantizeDirect } from './direct-runner';
import { quantizeErrorDiffusion } from './error-diffusion-runner';
import type { QuantizeAlgorithm, QuantizeAlgorithmContext } from './types';

function quantizeFloydSteinberg(context: QuantizeAlgorithmContext) {
	if (context.strength > 0) {
		quantizeErrorDiffusion(context);
		return;
	}
	quantizeDirect(context);
}

export const floydSteinbergAlgorithm: QuantizeAlgorithm = {
	id: 'floyd-steinberg',
	family: 'error-diffusion',
	quantize: quantizeFloydSteinberg
};
