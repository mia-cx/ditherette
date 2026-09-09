import { encodeOutput, usesIndexedWire } from './benchmark-indexed-wire.mjs';

/** Frozen-only Wasm execution in a disposable context. This module never imports the package or timers. */
export async function runTrial(trial) {
	const {
		default: init,
		evaluate,
		evaluate_prime
	} = await import('./oracle/ditherette_bench_oracle.js');
	await init({
		module_or_path: await (
			await fetch('/scripts/oracle/ditherette_bench_oracle_bg.wasm')
		).arrayBuffer()
	});
	const { source, rgba, identity, browser } = trial.case;
	const request = JSON.stringify({
		source,
		rgba,
		output: identity.output,
		operation: browser.operation,
		identity
	});
	const reference = JSON.parse(evaluate(request));
	const prime = browser.cache?.roles?.sample_prime;
	if (prime) reference.prime_output = JSON.parse(evaluate_prime(request, prime));
	if (usesIndexedWire(trial)) {
		if (prime) throw new Error('Capped indexed wire does not support stage priming.');
		reference.output = encodeOutput(reference.output);
	}
	return reference;
}
