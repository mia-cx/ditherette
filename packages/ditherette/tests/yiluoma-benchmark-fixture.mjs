/** Exercise actual public benchmark adapters without starting a timing collector. */
export async function yiluomaBenchmarkChecks({ vectors }) {
	const { prepareOperation, preflightOperation } =
		await import('/benchmark/benchmark-public-page.mjs');
	let calls = 0;
	for (const vector of vectors.cases) {
		for (const preparation of ['primed-instance', 'fresh-instance']) {
			const { source, palette, alpha, matching, dither } = vector.request;
			const dimensions = { width: source.width, height: source.height };
			const trial = {
				role: 'accepted',
				case: {
					source: dimensions,
					rgba: source.data,
					identity: { output: dimensions },
					measurement: {
						scope: 'complete-call',
						mode: 'latency',
						application_cache: 'not-applicable'
					},
					browser: {
						operation: {
							operation: 'yliluoma',
							settings: {
								quantize: { palette, alpha, matching },
								size: dither.size,
								placement: dither.placement
							}
						},
						accepted: 'package',
						candidate: 'package',
						preparation,
						cache: 'none'
					}
				},
				browser: {
					assets: {
						entries: {
							package: 'node_modules/ditherette/dist/index.js',
							wasm: 'node_modules/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm'
						}
					}
				}
			};
			const operation = await prepareOperation(trial);
			try {
				const output = vector.output;
				const mismatch = await preflightOperation(operation, {
					dimensions,
					pixels: {
						format: 'indexed8',
						indices: output.indices,
						palette_rgba: output.palette.rgba,
						transparent_index: output.palette.transparentIndex
					},
					warnings: output.warnings
				});
				if (mismatch)
					throw new Error(`Yliluoma adapter mismatch: ${matching}/${dither.size}/${preparation}`);
				calls++;
			} finally {
				operation.close();
			}
		}
	}
	return calls;
}
