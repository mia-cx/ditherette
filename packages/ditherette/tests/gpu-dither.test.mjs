import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
	createDitherette,
	createGpuDither,
	createGpuEffects,
	DitheretteError
} from '../dist/index.js';
import { initializeGpuDither } from '../dist/gpu-dither.js';

const wasm = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const options = { wasm, memoryLimitBytes: 32 * 1024 ** 2 };

test('request getters cannot dispose an entered GPU call and validation failures recover', async () => {
	// Proves normalisation holds the public active-call guard.
	const gpu = await createGpuDither(options);
	try {
		for (const method of ['ditherAndQuantize', 'process']) {
			const dither = request();
			const input =
				method === 'process'
					? {
							source: dither.source,
							palette: dither.palette,
							recipe: {
								version: 1,
								output: {
									width: 37,
									height: 19,
									resize: { algorithm: 'nearest', anchor: 'center' }
								},
								alpha: dither.alpha,
								match: dither.matching,
								dither: dither.dither
							}
						}
					: dither;
			const source = input.source;
			Object.defineProperty(input, 'source', {
				get() {
					assert.throws(
						() => gpu.dispose(),
						(error) => error.code === 'reentrant-call'
					);
					return source;
				}
			});
			await gpu[method](input);
			await assert.rejects(gpu[method]({}), (error) => error instanceof DitheretteError);
		}
		await gpu.ditherAndQuantize(request());
	} finally {
		gpu.dispose();
	}
});

test('abort from final progress rejects output and never publishes cold coverage', async () => {
	// Proves caller cancellation after GPU readback cannot publish reusable state.
	for (const stage of ['dither-and-quantize', 'complete']) {
		const input = request(),
			value = await expected(input),
			passed = [];
		const gpu = initializeGpuDither(
			{
				privateGpuDitherPlan: () => 1,
				privateGpuDitherDetect: async () => '',
				privateGpuEffectsAdapter: () => 'test adapter',
				privateGpuEffectsDispose() {},
				async privateGpuDitherAndQuantize(...args) {
					const sink = args.find((arg) => typeof arg?.check === 'function');
					passed.push(Boolean(args[args.indexOf(sink) - 3]));
					sink.value = value;
					sink.coverage = new Uint8Array(4 * Math.ceil((37 * 19) / 32));
					return '';
				}
			},
			options
		);
		try {
			const controller = new AbortController();
			input.onProgress = (progress) => {
				if (progress.stage === stage) controller.abort();
			};
			await assert.rejects(
				gpu.ditherAndQuantize(input, controller.signal),
				(error) => error.name === 'AbortError'
			);
			assert.equal(gpu.execution, undefined);
			delete input.onProgress;
			await gpu.ditherAndQuantize(input);
			await gpu.ditherAndQuantize(input);
			assert.deepEqual(passed, [false, false, true]);
		} finally {
			gpu.dispose();
		}
	}
});
const request = () => ({
	version: 1,
	source: {
		width: 37,
		height: 19,
		data: Uint8Array.from({ length: 37 * 19 * 4 }, (_, i) => (i * 61 + (i >> 2)) & 255)
	},
	palette: [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'transparent' }
	],
	alpha: { mode: 'preserve', threshold: 127.9 },
	matching: 'oklab-euclidean',
	dither: {
		family: 'separable',
		perturb: {
			field: { algorithm: 'bayer', size: '16' },
			space: 'oklab',
			strength: 0.3,
			placement: { mode: 'smooth-area-v1', sampleSize: 20, contrastThreshold: 0.12 }
		}
	}
});
async function expected(input) {
	const cpu = await createDitherette({ wasm });
	try {
		return cpu.ditherAndQuantize(input);
	} finally {
		cpu.dispose();
	}
}

test('GPU-only calls reject absent hardware and unsupported hybrid work without CPU fallback', async () => {
	// Proves a client can require GPU execution rather than silently receiving CPU work.
	const gpu = await createGpuDither({ ...options, fallback: 'error' });
	try {
		await assert.rejects(gpu.ditherAndQuantize(request()), (error) => error.code === 'capability');
		assert.equal(gpu.execution, undefined);
	} finally {
		gpu.dispose();
	}
	const effects = await createGpuEffects({ ...options, fallback: 'error' });
	try {
		await assert.rejects(
			effects.applyEffects({
				version: 1,
				source: request().source,
				effects: [{ effect: 'exposure', enabled: true, stops: 1 }]
			}),
			(error) => error.code === 'capability'
		);
	} finally {
		effects.dispose();
	}
	const unsupported = initializeGpuDither(
		{ privateGpuEffectsDispose() {} },
		{ ...options, fallback: 'error' }
	);
	try {
		const input = request();
		input.dither = {
			family: 'diffusion',
			kernel: 'sierra',
			feedback: 'matching',
			serpentine: true,
			strength: 1,
			placement: { mode: 'everywhere' }
		};
		await assert.rejects(
			unsupported.ditherAndQuantize(input),
			(error) => error.code === 'unsupported-operation'
		);
		assert.equal(unsupported.execution, undefined);
	} finally {
		unsupported.dispose();
	}
	await assert.rejects(
		createGpuDither({ ...options, fallback: false }),
		(error) => error.code === 'invalid-settings' && error.path === 'fallback'
	);
});

test('async indexed fallback preserves ordinary output, process recipes and owned storage', async () => {
	const processor = await createGpuDither(options);
	const input = request();
	const result = await processor.ditherAndQuantize(input);
	assert.deepEqual(result, await expected(input));
	assert.deepEqual(processor.execution, { backend: 'cpu', fallback: 'NoAdapter' });
	const process = {
		source: input.source,
		palette: input.palette,
		recipe: {
			version: 1,
			output: { width: 13, height: 11, resize: { algorithm: 'area' } },
			alpha: input.alpha,
			match: input.matching,
			dither: input.dither
		}
	};
	const cpu = await createDitherette({ wasm });
	try {
		assert.deepEqual(await processor.process(process), cpu.process(process));
	} finally {
		cpu.dispose();
	}
	processor.dispose();
	processor.dispose();
	assert.deepEqual(result, await expected(input));
	await assert.rejects(
		processor.ditherAndQuantize(input),
		(error) => error instanceof DitheretteError && error.code === 'disposed'
	);
});

test('async resize captures input before fallback and preserves identity and durable output', async () => {
	// Proves the async boundary preserves the existing resize ownership contract.
	const input = {
		version: 1,
		source: request().source,
		output: { width: 13, height: 11, resize: { algorithm: 'trilinear', anchor: 'bottom-right' } }
	};
	const cpu = await createDitherette({ wasm });
	const gpu = await createGpuDither({ ...options, tiling: { height: 3 } });
	try {
		const expected = cpu.resize(input);
		const pending = gpu.resize(input);
		input.source.data.fill(0);
		const actual = await pending;
		assert.deepEqual(actual, expected);
		assert.equal(gpu.execution.backend, 'cpu');
		const identity = {
			...input,
			output: { ...input.output, width: input.source.width, height: input.source.height }
		};
		assert.equal(await gpu.resize(identity), identity.source);
		gpu.dispose();
		assert.deepEqual(actual, expected);
	} finally {
		cpu.dispose();
		gpu.dispose();
	}
	const required = await createGpuDither({ ...options, fallback: 'error' });
	try {
		await assert.rejects(required.resize(input), (error) => error.code === 'capability');
	} finally {
		required.dispose();
	}
});

test('failed GPU initialization retires bindings before scalar allocation and preserves captured settings', async () => {
	let release;
	let retired = false;
	const processor = initializeGpuDither(
		{
			privateGpuDitherPlan: () => 1,
			privateGpuDitherDetect: () =>
				new Promise((resolve) => {
					release = resolve;
				}),
			privateGpuEffectsDispose: () => {
				retired = true;
			}
		},
		options
	);
	const input = request();
	const original = await expected(input);
	input.onProgress = () => assert.ok(retired, 'GPU ownership must end before scalar processing');
	const pending = processor.ditherAndQuantize(input);
	assert.throws(
		() => processor.dispose(),
		(error) => error.code === 'reentrant-call'
	);
	await assert.rejects(
		processor.ditherAndQuantize(input),
		(error) => error.code === 'reentrant-call'
	);
	input.source.data.fill(0);
	input.dither.perturb.strength = 0;
	input.palette[0].rgb = [255, 0, 0];
	release('NoAdapter');
	assert.deepEqual(await pending, original);
	processor.dispose();
});

test('aborted GPU jobs publish no result and the next job reuses the verified device', async () => {
	const input = request();
	const result = await expected(input);
	let detections = 0;
	let finish;
	let first = true;
	const processor = initializeGpuDither(
		{
			privateGpuDitherPlan: () => 1,
			privateGpuDitherDetect: async () => {
				detections++;
				return '';
			},
			privateGpuEffectsAdapter: () => 'test adapter',
			privateGpuEffectsDispose() {},
			async privateGpuDitherAndQuantize(...args) {
				const sink = args.at(-1);
				if (first) {
					first = false;
					await new Promise((resolve) => {
						finish = resolve;
					});
				}
				if (!sink.check()) return 'Cancelled';
				sink.value = result;
				return '';
			}
		},
		options
	);
	const controller = new AbortController();
	const pending = processor.ditherAndQuantize(input, controller.signal);
	await new Promise((resolve) => setImmediate(resolve));
	controller.abort();
	finish();
	await assert.rejects(pending, (error) => error.name === 'AbortError');
	assert.equal(processor.execution, undefined);
	assert.deepEqual(await processor.ditherAndQuantize(input), result);
	assert.equal(processor.execution.backend, 'gpu');
	assert.equal(detections, 1);
	processor.dispose();
});

test('coverage reuse verifies source bytes and controls and publishes only after successful completion', async () => {
	const input = request();
	const result = await expected(input);
	const passed = [];
	const processor = initializeGpuDither(
		{
			privateGpuDitherPlan: () => 1,
			privateGpuDitherDetect: async () => '',
			privateGpuEffectsAdapter: () => 'test adapter',
			privateGpuEffectsDispose() {},
			async privateGpuDitherAndQuantize(...args) {
				const coverage = args.at(-4);
				const retain = args.at(-3);
				const sink = args.at(-1);
				passed.push(Boolean(coverage));
				if (!coverage && retain)
					sink.coverage = new Uint8Array(
						4 * Math.ceil((input.source.width * input.source.height) / 32)
					);
				sink.value = result;
				return '';
			}
		},
		options
	);
	await processor.ditherAndQuantize(input);
	input.dither.perturb.strength = 1;
	input.matching = 'srgb-euclidean';
	input.palette[0].rgb = [255, 0, 0];
	await processor.ditherAndQuantize(input);
	input.source.data[0] ^= 1;
	await processor.ditherAndQuantize(input);
	input.dither.perturb.placement.sampleSize = 64;
	await processor.ditherAndQuantize(input);
	input.dither.perturb.placement.contrastThreshold = 0.06;
	input.onProgress = ({ stage }) => {
		if (stage === 'complete') throw Error('completion failed');
	};
	await assert.rejects(processor.ditherAndQuantize(input), (error) => error.code === 'callback');
	delete input.onProgress;
	await processor.ditherAndQuantize(input);
	assert.deepEqual(passed, [false, true, false, false, false, false]);
	assert.equal(processor.execution.analysisReused, false);
	await processor.ditherAndQuantize(input);
	assert.equal(processor.execution.analysisReused, true);
	processor.dispose();
});
