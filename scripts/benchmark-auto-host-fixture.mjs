import {
	assertMeasuredSource,
	equalOutput,
	prepareOperation,
	verificationOutput
} from './benchmark-public-page.mjs';
import { progressProbe } from './benchmark-progress.mjs';

/** One ordinary complete call per role, plus the selected field composition, without call timers. */
export async function runTrial(trial) {
	if (trial.case.browser.row_policy !== undefined)
		throw new Error('Automatic fixture forbids row overrides.');
	const OriginalWorker = globalThis.Worker;
	const instantiate = WebAssembly.instantiate;
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
	const call = async (request) => {
		const operation = await prepareOperation(request, () => {
			throw new Error('Unexpected private policy observation.');
		});
		try {
			const probe = progressProbe();
			const events = [];
			let insideCall = false;
			operation.request.onProgress = (event) => {
				if (!insideCall) throw new Error('Progress escaped the synchronous host call.');
				probe.onProgress(event);
				events.push({ ...event });
			};
			insideCall = true;
			const output = operation.call();
			insideCall = false;
			probe.verify();
			assertMeasuredSource(operation.request, request.case.rgba);
			return { output: verificationOutput(output), events };
		} finally {
			operation.close();
		}
	};
	try {
		const result = await call(trial);
		let composition;
		if (trial.composition) {
			const settings = trial.case.browser.operation.settings;
			const perturbed = await call({
				...trial,
				case: {
					...trial.case,
					browser: {
						...trial.case.browser,
						operation: { operation: 'perturb', settings: settings.perturb }
					}
				}
			});
			const indexed = await call({
				...trial,
				case: {
					...trial.case,
					rgba: perturbed.output.pixels.data,
					browser: {
						...trial.case.browser,
						operation: { operation: 'quantize', settings: settings.quantize }
					}
				}
			});
			composition = indexed.output;
			if (!equalOutput(result.output, composition))
				throw new Error('Automatic separable output differs from quantize(perturb).');
		}
		return {
			...result,
			composition,
			execution: globalThis instanceof DedicatedWorkerGlobalScope ? 'host-worker' : 'page',
			isolated: crossOriginIsolated,
			created,
			terminated,
			instantiation_restored: WebAssembly.instantiate === instantiate
		};
	} finally {
		globalThis.Worker = OriginalWorker;
	}
}
