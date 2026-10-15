import {
	smoothGradientChecks,
	smoothGradientBudgetChecks,
	coloredGradientChecks
} from './smooth-gradient-fixture.mjs';

Object.defineProperty(navigator, 'hardwareConcurrency', { value: 4, configurable: true });
self.onmessage = async ({ data: { moduleUrl } }) => {
	let scalar, threaded;
	try {
		const { createDitherette } = await import(moduleUrl);
		scalar = await createDitherette({ threads: 'disabled' });
		let workers = 0;
		const OriginalWorker = globalThis.Worker;
		globalThis.Worker = class extends OriginalWorker {
			constructor(...args) {
				super(...args);
				workers++;
			}
		};
		threaded = await createDitherette({ threads: 'required' });
		const reference = smoothGradientChecks(scalar);
		const actual = smoothGradientChecks(threaded);
		if (JSON.stringify(actual) !== JSON.stringify(reference))
			throw new Error('Scalar/threaded gradient output differs.');
		const colored = coloredGradientChecks(scalar);
		const threadedColored = coloredGradientChecks(threaded);
		if (JSON.stringify(colored) !== JSON.stringify(threadedColored))
			throw new Error('Scalar/threaded coloured texture differs.');
		if (workers === 0) throw new Error('Required threaded route started no workers.');
		const wasm = await WebAssembly.compile(
			await (
				await fetch(new URL('./wasm/threads/ditherette_wasm_bg.wasm', moduleUrl))
			).arrayBuffer()
		);
		const minima = await smoothGradientBudgetChecks((memoryLimitBytes) =>
			createDitherette({ threads: 'required', wasm, memoryLimitBytes })
		);
		if (minima.length !== 3) throw new Error('Threaded budget proof is incomplete.');
		self.postMessage({
			budgetMinima: minima,
			publicCases: actual.outputs.length,
			exactScalarThreaded: true,
			repeatedPlateaus: true,
			actualWorkers: workers > 0,
			coloredTextureCases: colored.outputs.length
		});
	} catch (error) {
		self.postMessage({ error: String(error), stack: error.stack });
	} finally {
		scalar?.dispose();
		threaded?.dispose();
	}
};
