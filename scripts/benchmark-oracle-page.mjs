/** Frozen-only Wasm execution in a disposable context. This module never imports the package or timers. */
export async function runTrial(trial) {
	const { default: init, evaluate } = await import('./oracle/ditherette_bench_oracle.js');
	await init({
		module_or_path: await (
			await fetch('/scripts/oracle/ditherette_bench_oracle_bg.wasm')
		).arrayBuffer()
	});
	const { source, rgba, identity, browser } = trial.case;
	return JSON.parse(
		evaluate(
			JSON.stringify({
				source,
				rgba,
				output: identity.output,
				operation: browser.operation,
				identity
			})
		)
	);
}
