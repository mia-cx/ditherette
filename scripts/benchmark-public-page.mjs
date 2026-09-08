import {
	collectCalls,
	collectInitializations,
	retainedOutputSlots
} from './benchmark-public-timing.mjs';
import { prepareStageSample, stagePrimeRequest } from './benchmark-stage-cache.mjs';
import { progressProbe } from './benchmark-progress.mjs';

/** Observe the browser clock quantum without changing, retrying, or censoring operation samples. */
export function timerResolution(now = () => performance.now()) {
	let previous = now();
	let quantum = Infinity;
	for (let index = 0; index < 100_000; index += 1) {
		const current = now();
		if (current > previous) quantum = Math.min(quantum, current - previous);
		previous = current;
	}
	if (!Number.isFinite(quantum)) throw new Error('Could not observe browser timer resolution.');
	return quantum * 1e6;
}

/** Prepare only existing public operations. */
export function resizeRecipe(operation) {
	switch (operation.operation) {
		case 'resize-nearest':
			return { algorithm: 'nearest', anchor: operation.anchor };
		case 'resize-area':
			return { algorithm: 'area' };
		case 'resize-bilinear':
			return { algorithm: 'bilinear', anchor: operation.anchor };
		case 'resize-trilinear':
			return { algorithm: 'trilinear', anchor: operation.anchor };
		case 'resize-bicubic':
		case 'resize-lanczos2':
		case 'resize-lanczos3':
			return {
				algorithm: operation.operation.slice('resize-'.length),
				anchor: operation.anchor,
				support: operation.support
			};
		default:
			throw new Error('Unsupported browser operation.');
	}
}

/** Prepare the actual package or website call outside measurement timers. */
export async function prepareOperation(trial) {
	const config = trial.case.browser;
	const execution =
		typeof DedicatedWorkerGlobalScope !== 'undefined' &&
		globalThis instanceof DedicatedWorkerGlobalScope
			? 'host-worker'
			: 'page';
	if ((config.execution ?? 'page') !== execution)
		throw new Error('Browser execution context differs from the declaration.');
	const backend = config[trial.role];
	const measurement = trial.case.measurement;
	if (
		config.threads !== undefined &&
		(!config.threads ||
			!['disabled', 'preferred', 'required'].includes(config.threads.accepted) ||
			!['disabled', 'preferred', 'required'].includes(config.threads.candidate) ||
			config.accepted !== 'package' ||
			config.candidate !== 'package')
	)
		throw new Error('Thread policies require ordinary package calls and valid role policies.');
	const threads = config.threads?.[trial.role] ?? 'disabled';
	if (
		config.progress !== undefined &&
		(!config.progress ||
			!['disabled', 'enabled'].includes(config.progress.accepted) ||
			!['disabled', 'enabled'].includes(config.progress.candidate) ||
			config.accepted !== 'package' ||
			config.candidate !== 'package' ||
			config.preparation !== 'fresh-instance' ||
			measurement.mode !== 'single-call' ||
			measurement.scope !== 'complete-call' ||
			measurement.application_cache !== 'cold')
	)
		throw new Error('Progress comparisons require cold single ordinary package calls.');
	const quantize = config.operation.operation === 'quantize';
	const perturb = config.operation.operation === 'perturb';
	const separable = config.operation.operation === 'separable';
	const yliluoma = config.operation.operation === 'yliluoma';
	const diffusion = config.operation.operation === 'diffusion';
	const process = config.operation.operation === 'process';
	const resize =
		process || quantize || perturb || separable || diffusion || yliluoma
			? undefined
			: resizeRecipe(config.operation);
	const cacheComparison = config.cache !== 'none';
	const samplePrime = config.cache?.roles?.sample_prime;
	if (cacheComparison) {
		const roles = config.cache?.roles;
		const stages = [roles?.accepted, roles?.candidate].includes('image-stages');
		if (
			!roles ||
			!['uncached', 'preparation', 'image-stages'].includes(roles.accepted) ||
			!['uncached', 'preparation', 'image-stages'].includes(roles.candidate) ||
			Boolean(samplePrime) !== (stages && measurement.application_cache === 'warm') ||
			measurement.mode !== 'single-call' ||
			measurement.scope !== 'complete-call' ||
			!['cold', 'warm'].includes(measurement.application_cache) ||
			config.preparation !==
				(measurement.application_cache === 'cold'
					? 'fresh-instance'
					: samplePrime
						? 'primed-sample'
						: 'primed-instance') ||
			config.accepted !== 'package' ||
			config.candidate !== 'package'
		)
			throw new Error('Invalid preparation-cache comparison.');
	} else if (measurement.application_cache !== 'not-applicable') {
		throw new Error('Application cache claims require role capabilities.');
	}
	if (measurement.mode === 'throughput' && config.preparation !== 'primed-instance')
		throw new Error('Throughput requires a primed instance.');
	const request = process
		? {
				source: { ...trial.case.source, data: new Uint8Array(trial.case.rgba) },
				...config.operation.settings
			}
		: {
				version: 1,
				source: { ...trial.case.source, data: new Uint8Array(trial.case.rgba) },
				...(perturb
					? { perturb: config.operation.settings }
					: separable || diffusion || yliluoma
						? {
								...config.operation.settings.quantize,
								dither: diffusion
									? {
											family: 'diffusion',
											kernel: config.operation.settings.kernel,
											feedback: config.operation.settings.feedback,
											strength: config.operation.settings.strength,
											serpentine: config.operation.settings.serpentine,
											placement: config.operation.settings.placement
										}
									: yliluoma
										? {
												family: 'yliluoma',
												size: config.operation.settings.size,
												placement: config.operation.settings.placement
											}
										: { family: 'separable', perturb: config.operation.settings.perturb }
							}
						: quantize
							? config.operation.settings
							: {
									output: {
										...trial.case.identity.output,
										resize
									}
								})
			};
	const progress = config.progress?.[trial.role] === 'enabled' ? progressProbe() : undefined;
	if (progress) request.onProgress = progress.onProgress;
	const url = (entry) => new URL(`/${entry}`, location.href).href;
	if (backend === 'typescript') {
		if (process) throw new Error('No faithful TypeScript Process adapter is registered.');
		if (yliluoma) throw new Error('No faithful TypeScript Yliluoma adapter is registered.');
		if (perturb || separable || diffusion)
			throw new Error('No faithful TypeScript field adapter is registered.');
		if (quantize) throw new Error('No faithful TypeScript indexed quantize adapter is registered.');
		if (resize.algorithm === 'bicubic' || resize.algorithm === 'trilinear')
			throw new Error('The website has no bicubic or trilinear implementation.');
		if ('anchor' in resize && resize.anchor !== 'center')
			throw new Error('TypeScript non-center resize is unavailable.');
		if (
			measurement.scope !== 'complete-call' ||
			!['primed-instance', 'fresh-instance'].includes(config.preparation)
		)
			throw new Error('TypeScript has no Wasm initialization or processor-instance equivalent.');
		// TypeScript is stateless: both labels execute its ordinary per-call preparation.
		const { resize: resizeTypeScript } = await import(url(trial.browser.assets.entries.typescript));
		return {
			request,
			call: () => resizeTypeScript(request),
			prepare: async () => ({ call: () => resizeTypeScript(request), close() {} }),
			close() {}
		};
	}
	if (backend !== 'package' && !(backend === 'package-staged' && process))
		throw new Error('Unknown browser backend.');
	const { createDitherette } = await import(url(trial.browser.assets.entries.package));
	const response = await fetch(url(trial.browser.assets.entries.wasm));
	if (!response.ok) throw new Error(`Wasm fetch failed: ${response.status}`);
	const bytes = await response.arrayBuffer();
	// Load the real lazy factory module before timing. Browser compilation caches are not reset.
	const preload = await createDitherette({ wasm: bytes, threads });
	preload.dispose();
	const compiled =
		config.preparation === 'initialization-bytes' ? undefined : await WebAssembly.compile(bytes);
	const create = () => createDitherette({ wasm: compiled ?? bytes, threads });
	const call = process
		? (instance) =>
				backend === 'package-staged'
					? instance.ditherAndQuantize({
							version: 1,
							source: instance.resize({
								version: 1,
								source: request.source,
								output: request.recipe.output
							}),
							palette: request.palette,
							alpha: request.recipe.alpha,
							matching: request.recipe.match,
							dither: request.recipe.dither
						})
					: instance.process(request)
		: perturb
			? (instance) => instance.perturb(request)
			: separable || diffusion || yliluoma
				? (instance) => instance.ditherAndQuantize(request)
				: quantize
					? (instance) => instance.quantize(request)
					: (instance) => instance.resize(request);
	if (measurement.scope === 'initialization') {
		if (!['initialization-bytes', 'initialization-compiled'].includes(config.preparation))
			throw new Error('Initialization requires an explicit compilation scope.');
		return { request, create, probe: call, close() {} };
	}
	if (measurement.scope !== 'complete-call')
		throw new Error('Browser processing requires complete-call scope.');
	if (config.preparation === 'primed-sample') {
		const prime = stagePrimeRequest(config.operation.operation, request, samplePrime);
		if (!trial.prime_reference_output)
			throw new Error('Stage priming requires a frozen prime output.');
		return {
			request,
			prepare: () =>
				prepareStageSample({
					create,
					prime: (instance) => (prime ? instance[prime.method](prime.request) : call(instance)),
					call,
					observePrime: (output) => {
						assertMeasuredSource(request, trial.case.rgba);
						const actual = verificationOutput(output);
						if (!equalOutput(actual, trial.prime_reference_output)) {
							throw new Error(
								`Stage prime differs from frozen reference: ${JSON.stringify({ expected: trial.prime_reference_output, actual })}`
							);
						}
					}
				}),
			close() {}
		};
	}
	if (config.preparation === 'fresh-instance') {
		return {
			request,
			observe: progress?.verify,
			prepare: async () => {
				const instance = await create();
				progress?.reset();
				return { call: () => call(instance), close: () => instance.dispose() };
			},
			close() {}
		};
	}
	if (config.preparation !== 'primed-instance') throw new Error('Unknown processing preparation.');
	const instance = await create();
	if (cacheComparison) {
		try {
			primeChangedSource(request, () => call(instance));
		} catch (error) {
			instance.dispose();
			throw error;
		}
	}
	return {
		request,
		call: () => call(instance),
		prepare: async () => ({ call: () => call(instance), close() {} }),
		close: () => instance.dispose()
	};
}

/** Prime preparation with different source bytes, then restore the measured request. */
export function primeChangedSource(request, call) {
	const measured = request.source.data;
	const prime = measured.slice();
	for (let index = 0; index < prime.length; index += 4) prime[index] ^= 0xff;
	request.source.data = prime;
	try {
		const output = call();
		const indexed = 'indices' in output;
		const bytes =
			output.width * output.height * (indexed ? 1 : 4) + (indexed ? MAX_PALETTE_BYTES : 0);
		outputStability(bytes, indexed ? 'indexed8' : 'rgba8').observe([output]);
	} finally {
		request.source.data = measured;
	}
}

/** Check every observed call so a later call cannot hide transient input mutation. */
export function assertMeasuredSource(request, rgba) {
	if (
		request.source.data.length !== rgba.length ||
		!request.source.data.every((byte, index) => byte === rgba[index])
	)
		throw new Error('Operation mutated source bytes.');
}

// Comparison views share buffers. Stability snapshots own separate typed storage.
function outputView(output) {
	if ('indices' in output) {
		return {
			dimensions: { width: output.width, height: output.height },
			pixels: {
				format: 'indexed8',
				indices: output.indices,
				palette_rgba: output.palette.rgba,
				transparent_index: output.palette.transparentIndex
			},
			warnings: output.warnings
		};
	}
	return {
		dimensions: { width: output.width, height: output.height },
		pixels: { format: 'rgba8', data: output.data },
		warnings: []
	};
}

export function verificationOutput(output) {
	const view = outputView(output);
	return {
		...view,
		pixels:
			view.pixels.format === 'rgba8'
				? { ...view.pixels, data: Array.from(view.pixels.data) }
				: {
						...view.pixels,
						indices: Array.from(view.pixels.indices),
						palette_rgba: Array.from(view.pixels.palette_rgba)
					},
		warnings: view.warnings.map(({ code, message }) => ({ code, message }))
	};
}

const MAX_PALETTE_BYTES = 256 * 4;
const MAX_WARNINGS = 3;
const MAX_WARNING_CHARACTERS = 88;
const WARNING_CODES = new Set(['palette-truncated', 'transparent-only', 'transparent-fallback']);

/** Require independent records and durable buffers; retain only the first and first distinct result. */
export function outputStability(outputBytes, format = 'rgba8') {
	let first;
	let firstSnapshot;
	let distinct;
	const retainedStorage = new Set();
	return {
		observe(outputs) {
			const storage = new Set();
			for (const output of outputs) {
				const indexed = format === 'indexed8';
				const actualIndexed = 'indices' in output;
				if (indexed !== actualIndexed) throw new Error('Unexpected public output format.');
				const views = indexed ? [output.indices, output.palette.rgba] : [output.data];
				const pixels = indexed ? outputBytes - MAX_PALETTE_BYTES : outputBytes;
				for (const [index, data] of views.entries()) {
					if (
						!ArrayBuffer.isView(data) ||
						!(data.buffer instanceof ArrayBuffer) || // Cloning shared storage does not copy its bytes.
						data.BYTES_PER_ELEMENT !== 1 ||
						data.buffer.byteLength !== data.byteLength ||
						(index === 0
							? data.byteLength !== pixels
							: data.byteLength < 4 ||
								data.byteLength > MAX_PALETTE_BYTES ||
								data.byteLength % 4 !== 0)
					)
						throw new Error('Output storage violates the declared durable byte-result contract.');
				}
				if (indexed) {
					const count = output.palette.rgba.length / 4;
					const transparent = output.palette.transparentIndex;
					if (
						(transparent !== null &&
							(!Number.isInteger(transparent) || transparent < 0 || transparent >= count)) ||
						output.indices.some((index) => index >= count) ||
						!Array.isArray(output.warnings) ||
						output.warnings.length > MAX_WARNINGS ||
						output.warnings.some(
							(warning) =>
								!WARNING_CODES.has(warning.code) ||
								typeof warning.message !== 'string' ||
								warning.message.length > MAX_WARNING_CHARACTERS ||
								Object.keys(warning).some((key) => !['code', 'message'].includes(key))
						) ||
						Object.keys(output).some(
							(key) => !['width', 'height', 'indices', 'palette', 'warnings'].includes(key)
						) ||
						Object.keys(output.palette).some((key) => !['rgba', 'transparentIndex'].includes(key))
					)
						throw new Error('Indexed metadata exceeds the declared public result contract.');
				}
				const records = indexed
					? [output, output.palette, output.warnings, ...output.warnings]
					: [output];
				const owned = [...records, ...views.map((data) => data.buffer)];
				for (const value of owned) {
					if (storage.has(value) || retainedStorage.has(value))
						throw new Error('Output storage aliases an earlier retained result.');
					storage.add(value);
				}
				if (!first) {
					first = output;
					firstSnapshot = structuredClone(output);
					for (const value of owned) retainedStorage.add(value);
					continue;
				}
				if (!distinct && !equalOutput(outputView(first), outputView(firstSnapshot)))
					distinct = structuredClone(first);
				if (!distinct && !equalOutput(outputView(output), outputView(firstSnapshot))) {
					distinct = structuredClone(output);
					for (const value of owned) retainedStorage.add(value);
				}
			}
		},
		evidence(finalOutput) {
			return distinct
				? {
						unstable_output: verificationOutput(firstSnapshot),
						output: verificationOutput(distinct)
					}
				: { output: verificationOutput(finalOutput) };
		}
	};
}

/** Compare one untimed actual call with worker-supplied frozen bytes; return concrete mismatch evidence. */
export async function preflightOperation(operation, reference, observe = () => {}) {
	if (!reference || !['rgba8', 'indexed8'].includes(reference.pixels.format))
		throw new Error('Browser trial requires frozen RGBA8 or indexed reference_output.');
	let output;
	if (operation.create) {
		const instance = await operation.create();
		try {
			output = operation.probe(instance);
			observe([output]);
		} finally {
			instance.dispose();
		}
	} else {
		const prepared = await operation.prepare();
		try {
			output = prepared.call();
			observe([output]);
		} finally {
			prepared.close();
		}
	}
	const actual = verificationOutput(output);
	if (equalOutput(actual, reference)) return undefined;
	return actual;
}

/** Exact bytes and metadata, independent of JSON object key order. */
export function equalOutput(actual, expected) {
	const equalBytes = (a, b) => a.length === b.length && a.every((byte, index) => byte === b[index]);
	if (
		actual.dimensions.width !== expected.dimensions.width ||
		actual.dimensions.height !== expected.dimensions.height ||
		actual.pixels.format !== expected.pixels.format ||
		actual.warnings.length !== expected.warnings.length ||
		actual.warnings.some(
			(warning, index) =>
				warning.code !== expected.warnings[index].code ||
				warning.message !== expected.warnings[index].message
		)
	)
		return false;
	if (actual.pixels.format === 'rgba8') return equalBytes(actual.pixels.data, expected.pixels.data);
	if (actual.pixels.format !== 'indexed8') return false;
	return (
		equalBytes(actual.pixels.indices, expected.pixels.indices) &&
		equalBytes(actual.pixels.palette_rgba, expected.pixels.palette_rgba) &&
		actual.pixels.transparent_index === expected.pixels.transparent_index
	);
}

/** Fail before timing when the equivalent production composition differs; retain both concrete outputs. */
export async function requireMatchingComposition(operation, current) {
	const counterpart = await preflightOperation(operation, current);
	if (counterpart)
		throw new Error(
			`Process differs from staged production before timing: ${JSON.stringify({ current, counterpart })}`
		);
}

/** Invoked only by the leased transport. All serialization and observations are outside call timers. */
export async function runTrial(trial) {
	const resolution = timerResolution();
	const operation = await prepareOperation(trial);
	try {
		const identity = {
			role: trial.role,
			pair: trial.pair,
			case_name: trial.case.name,
			input: trial.case.identity.input,
			settings: trial.case.identity.settings
		};
		const observation = {
			...(trial.case.browser.execution === undefined
				? {}
				: { execution: trial.case.browser.execution }),
			user_agent: navigator.userAgent,
			cross_origin_isolated: crossOriginIsolated,
			timer_resolution_ns: resolution
		};
		const format = ['quantize', 'separable', 'diffusion', 'yliluoma', 'process'].includes(
			trial.case.browser.operation.operation
		)
			? 'indexed8'
			: 'rgba8';
		const pixels = trial.case.identity.output.width * trial.case.identity.output.height;
		// Indexed records reserve the maximum palette plus the collector's fixed metadata allowance.
		const outputBytes = format === 'indexed8' ? pixels + MAX_PALETTE_BYTES : pixels * 4;
		retainedOutputSlots(1, outputBytes); // Bound the first probe and retained evidence before producing either.
		const stability = outputStability(outputBytes, format);
		const observe = (outputs) => {
			stability.observe(outputs);
			assertMeasuredSource(operation.request, trial.case.rgba);
			operation.observe?.();
		};
		const mismatch = await preflightOperation(operation, trial.reference_output, observe);
		if (trial.case.browser.operation.operation === 'process') {
			const comparison = await prepareOperation({
				...trial,
				case: {
					...trial.case,
					browser: {
						...trial.case.browser,
						cache: 'none',
						progress: undefined,
						preparation:
							trial.case.browser.preparation === 'primed-sample'
								? 'fresh-instance'
								: trial.case.browser.preparation,
						accepted: 'package-staged',
						candidate: 'package'
					},
					measurement: { ...trial.case.measurement, application_cache: 'not-applicable' }
				},
				role: trial.case.browser[trial.role] === 'package' ? 'accepted' : 'candidate'
			});
			try {
				const current = mismatch ?? trial.reference_output;
				await requireMatchingComposition(comparison, current);
			} finally {
				comparison.close();
			}
		}
		if (mismatch && trial.case.browser.measure_nonexact !== true)
			return {
				...identity,
				sample_ns: [],
				iterations_per_sample: 0,
				warmup_iterations: 0,
				warmup_elapsed_ns: 0,
				output: mismatch,
				observation,
				timing_skipped: 'reference-mismatch'
			};
		const measurement = trial.case.measurement;
		const measured =
			measurement.scope === 'initialization'
				? await collectInitializations({
						measurement,
						create: operation.create,
						probe: operation.probe,
						observe,
						outputBytes
					})
				: await collectCalls({
						measurement,
						prepare: operation.prepare,
						observe,
						outputBytes
					});
		const { output, ...timings } = measured;
		// Keep the actual timing result separate. Instability evidence is the first distinct pair,
		// not a claim that the final timed output still differs (A/B/A must also fail).
		const evidence = stability.evidence(output);
		return {
			...identity,
			...timings,
			...evidence,
			observation
		};
	} finally {
		operation.close();
	}
}
