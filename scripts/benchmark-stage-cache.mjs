/** Construct a warm-stage prime from an ordinary measured public request. */
export function stagePrimeRequest(operation, request, prime) {
	switch (prime) {
		case 'same-call':
			return undefined;
		case 'resize':
			if (operation !== 'process') break;
			return {
				method: 'resize',
				request: { version: 1, source: request.source, output: request.recipe.output }
			};
		case 'perturb':
			if (operation !== 'separable') break;
			return {
				method: 'perturb',
				request: { version: 1, source: request.source, perturb: request.dither.perturb }
			};
		case 'no-dither':
			if (operation !== 'quantize') break;
			return {
				method: 'ditherAndQuantize',
				request: { ...request, dither: { family: 'none' } }
			};
	}
	throw new Error('Stage prime does not match the measured operation.');
}

/** Prime and observe a fresh instance before exposing its single timed call. */
export async function prepareStageSample({ create, prime, call, observePrime }) {
	const instance = await create();
	try {
		const output = prime(instance);
		observePrime(output);
		return { call: () => call(instance), close: () => instance.dispose() };
	} catch (error) {
		instance.dispose();
		throw error;
	}
}
