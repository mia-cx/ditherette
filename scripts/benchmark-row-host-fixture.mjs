import { prepareOperation, verificationOutput } from './benchmark-public-page.mjs';
import { progressProbe } from './benchmark-progress.mjs';

/** Run the actual complete-call preparation and public method without collecting timings. */
export async function runTrial(trial) {
	const instantiate = WebAssembly.instantiate;
	let rowPolicy;
	const operation = await prepareOperation(trial, (observation) => {
		rowPolicy = observation;
	});
	try {
		if (WebAssembly.instantiate !== instantiate)
			throw new Error('Instantiation hook leaked into public calls.');
		const probe = progressProbe();
		let events = [];
		let insideCall = false;
		let failCompletion = false;
		operation.request.onProgress = (event) => {
			if (!insideCall) throw new Error('Progress escaped its synchronous host call.');
			probe.onProgress(event);
			events.push({ ...event });
			if (failCompletion && event.stage === 'complete')
				throw new Error('row fixture callback failure');
		};
		const call = () => {
			events = [];
			probe.reset();
			insideCall = true;
			try {
				return operation.call();
			} finally {
				insideCall = false;
			}
		};
		const original = call();
		probe.verify();
		const output = verificationOutput(original);
		const firstEvents = events;
		// A new source bypasses the successful stage cache before failing at final completion.
		operation.request.source.data[0] ^= 255;
		failCompletion = true;
		let failure;
		try {
			call();
		} catch (error) {
			failure = { code: error.code, path: error.path };
		}
		if (!failure) throw new Error('Completion callback failure returned a result.');
		const failedEvents = events;
		failCompletion = false;
		const recovered = verificationOutput(call());
		probe.verify();
		const recoveryEvents = events;
		const repeated = verificationOutput(call());
		probe.verify();
		return {
			execution: globalThis instanceof DedicatedWorkerGlobalScope ? 'host-worker' : 'page',
			isolated: crossOriginIsolated,
			row_policy: rowPolicy,
			output,
			first_events: firstEvents,
			failure,
			failed_events: failedEvents,
			recovered,
			recovery_events: recoveryEvents,
			repeated,
			original_after_calls: verificationOutput(original),
			instantiation_restored: WebAssembly.instantiate === instantiate
		};
	} finally {
		operation.close();
	}
}
