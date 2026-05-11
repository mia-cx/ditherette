import type { QuantizeAlgorithm, QuantizeAlgorithmContext } from './types';

function quantizeErrorDiffusionAlgorithm(context: QuantizeAlgorithmContext) {
	if (context.strength > 0) {
		context.runErrorDiffusion();
		return;
	}
	context.runDirect();
}

export const floydSteinbergAlgorithm: QuantizeAlgorithm = {
	id: 'floyd-steinberg',
	family: 'error-diffusion',
	quantize: quantizeErrorDiffusionAlgorithm
};

export const sierraAlgorithm: QuantizeAlgorithm = {
	id: 'sierra',
	family: 'error-diffusion',
	quantize: quantizeErrorDiffusionAlgorithm
};

export const sierraLiteAlgorithm: QuantizeAlgorithm = {
	id: 'sierra-lite',
	family: 'error-diffusion',
	quantize: quantizeErrorDiffusionAlgorithm
};
