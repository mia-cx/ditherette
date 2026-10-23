import { areaChecks, areaBudgetChecks, featherChecks } from './smooth-area-fixture.mjs';

Object.defineProperty(navigator, 'hardwareConcurrency', { value: 4, configurable: true });
const moduleUrl = new URL('../dist/index.js', import.meta.url).href;
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
	const expected = areaChecks(scalar),
		actual = areaChecks(threaded);
	if (JSON.stringify(expected) !== JSON.stringify(actual))
		throw Error('Scalar/threaded area output differs.');
	if (JSON.stringify(featherChecks(scalar)) !== JSON.stringify(featherChecks(threaded)))
		throw Error('Scalar/threaded boundary coverage differs.');
	if (workers === 0) throw Error('Required threaded route started no workers.');
	const wasm = await WebAssembly.compile(
		await (await fetch(new URL('./wasm/threads/ditherette_wasm_bg.wasm', moduleUrl))).arrayBuffer()
	);
	const budgetMinima = await areaBudgetChecks((memoryLimitBytes) =>
		createDitherette({ threads: 'required', wasm, memoryLimitBytes })
	);
	self.postMessage({
		budgetMinima,
		publicCases: actual.outputs.length,
		exactScalarThreaded: true,
		completeFootprints: actual.completeFootprints,
		actualWorkers: workers > 0
	});
} catch (error) {
	self.postMessage({ error: String(error), stack: error.stack });
} finally {
	scalar?.dispose();
	threaded?.dispose();
}
