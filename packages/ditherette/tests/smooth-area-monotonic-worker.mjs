import { createDitherette } from '../dist/index.js';
import { adjacentToleranceChecks } from './smooth-area-fixture.mjs';

Object.defineProperty(navigator, 'hardwareConcurrency', { value: 4, configurable: true });
let scalar, threaded;
try {
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
	const expected = adjacentToleranceChecks(scalar);
	const actual = adjacentToleranceChecks(threaded);
	if (JSON.stringify(expected) !== JSON.stringify(actual))
		throw Error('Scalar/threaded adjacent-tolerance output differs.');
	if (workers === 0) throw Error('Required threaded route started no workers.');
	self.postMessage({ exactScalarThreaded: true, monotoneSelection: true, actualWorkers: true });
} catch (error) {
	self.postMessage({ error: String(error), stack: error.stack });
} finally {
	scalar?.dispose();
	threaded?.dispose();
}
