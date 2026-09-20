import {
	prepareOperation,
	runTrial as measuredTrial,
	verificationOutput
} from './benchmark-public-page.mjs';

/** Exercise host wiring with either a fake clock or untimed real initialization. Never time the installed package. */
export async function runTrial(trial) {
	if (trial.fake) {
		let tick = 0;
		Object.defineProperty(globalThis, 'performance', { value: { now: () => ++tick } });
		const fixture = await import('./benchmark-stage-cache-fixture.mjs');
		fixture.reset();
		if (trial.fail) fixture.failInitialization(3);
		const result = await measuredTrial(trial);
		return { ...result, fixture_events: fixture.events };
	}
	const OriginalWorker = globalThis.Worker;
	let created = 0;
	let terminated = 0;
	globalThis.Worker = class extends OriginalWorker {
		constructor(...args) {
			super(...args);
			created++;
		}
		terminate() {
			terminated++;
			super.terminate();
		}
	};
	try {
		const operation = await prepareOperation(trial);
		const outputs = [];
		try {
			for (let index = 0; index < 3; index++) {
				const instance = await operation.create();
				try {
					outputs.push(verificationOutput(operation.probe(instance)));
				} finally {
					instance.dispose();
				}
			}
		} finally {
			operation.close();
		}
		return { outputs, created, terminated };
	} finally {
		globalThis.Worker = OriginalWorker;
	}
}
