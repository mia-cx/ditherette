const stages = { resize: 0, indexed: 1, mixing: 2 };

/** Set a developer policy on the instance created by the unchanged public factory, outside timing. */
export async function createRowBandProcessor(create, policy, availableCpus, wasm = WebAssembly) {
	const { stage, parameters } = policy;
	if (
		!Object.hasOwn(stages, stage) ||
		!Number.isInteger(parameters.height) || parameters.height < 0 || parameters.height > 32768 ||
		!Number.isInteger(parameters.active_workers) || parameters.active_workers < 1 || parameters.active_workers > 8
	) throw new Error('Invalid developer row policy.');
	const instantiate = wasm.instantiate;
	const instances = [];
	let processor;
	try {
		// The runner supplies a compiled Module. Capture only this host's ordinary instantiation.
		wasm.instantiate = async function (...args) {
			const result = await Reflect.apply(instantiate, wasm, args);
			instances.push(result.instance ?? result);
			return result;
		};
		try {
			processor = await create();
		} finally {
			wasm.instantiate = instantiate;
		}
		if (instances.length !== 1) throw new Error('Expected one public processor Wasm instance.');
		const bindings = instances[0].exports;
		if (typeof bindings.privateExecutionPolicy !== 'function' || typeof bindings.privateThreadCount !== 'function')
			throw new Error('Row comparisons require the explicit benchmark-feature threaded artifact.');
		const pool_size = bindings.privateThreadCount(availableCpus);
		if (!Number.isInteger(pool_size) || pool_size < 1 || pool_size > 8 || parameters.active_workers > pool_size)
			throw new Error('Requested row workers exceed the actual pool capacity.');
		const status = bindings.privateExecutionPolicy(stages[stage], parameters.height, parameters.active_workers, pool_size);
		if (status !== 0) throw new Error(`Private row policy failed with status ${status}.`);
		return { processor, observation: { stage, parameters: { ...parameters }, pool_size } };
	} catch (error) {
		processor?.dispose();
		throw error;
	}
}
