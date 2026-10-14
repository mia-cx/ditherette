import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette, createGpuEffects, DitheretteError } from '../dist/index.js';
import { initializeGpuEffects } from '../dist/gpu-effects.js';

const wasm = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const effects = [
	{ effect: 'brightness-contrast', enabled: true, brightness: -0.11, contrast: 0.32 },
	{ effect: 'exposure', enabled: true, stops: 0.6 },
	{ effect: 'white-balance', enabled: true, temperature: -0.2, tint: 0.4 }
];
const source = () => ({
	width: 37,
	height: 19,
	data: Uint8Array.from({ length: 37 * 19 * 4 }, (_, i) => (i * 61 + (i >> 2)) & 255)
});
const request = (extra = {}) => ({ version: 1, source: source(), effects, ...extra });
const options = { wasm, memoryLimitBytes: 32 * 1024 ** 2 };
const diagnostic = (code) => (error) => error instanceof DitheretteError && error.code === code;

async function expected(input) {
	const cpu = await createDitherette({ wasm });
	try {
		return cpu.applyEffects(input);
	} finally {
		cpu.dispose();
	}
}

test('public async API uses real scalar fallback without WebGPU and retains independent outputs', async () => {
	const processor = await createGpuEffects(options);
	const input = request();
	const original = input.source.data.slice();
	const result = await processor.applyEffects(input);
	assert.deepEqual(result, await expected(input));
	assert.deepEqual(processor.execution, { backend: 'cpu', fallback: 'NoAdapter' });
	assert.deepEqual(input.source.data, original);
	assert.notEqual(result.data.buffer, input.source.data.buffer);
	await processor.applyEffects(
		request({ source: { ...input.source, data: new Uint8Array(original.length) } })
	);
	processor.dispose();
	processor.dispose();
	assert.deepEqual(result, await expected(input));
	await assert.rejects(processor.applyEffects(input), diagnostic('disposed'));
});

test('GPU failures release resources before the existing CPU fallback, including throwing artifacts', async () => {
	for (const reason of [
		'Lost(device destroyed)',
		'Failed(map)',
		'TooLarge',
		'MemoryLimit',
		'throw'
	]) {
		const events = [];
		const bindings = {
			privateGpuEffectsPlanningBytes: () => 1,
			privateGpuEffectsPlan: () => 1,
			privateGpuEffectsDetect: async () => '',
			privateGpuEffectsAdapter: () => 'test adapter',
			privateGpuApplyEffects: async () => {
				if (reason === 'throw') throw Error('trap');
				return reason;
			},
			privateGpuEffectsDispose: () => events.push('release')
		};
		const processor = initializeGpuEffects(bindings, options);
		const input = request({
			onProgress: ({ stage }) => {
				assert.equal(events[0], 'release');
				if (stage === 'prepare') events.push('CPU');
			}
		});
		const result = await processor.applyEffects(input);
		assert.deepEqual(result, await expected(request({ source: input.source })));
		assert.equal(processor.execution.backend, 'cpu');
		assert.equal(processor.execution.fallback, reason === 'throw' ? 'Failed' : reason);
		processor.dispose();
	}
});

test('known-answer rejection stays on CPU for the facade lifetime', async () => {
	let detections = 0;
	const processor = initializeGpuEffects(
		{
			privateGpuEffectsPlanningBytes: () => 1,
			privateGpuEffectsPlan: () => 1,
			privateGpuEffectsDetect: async () => {
				detections++;
				return 'Mismatch';
			},
			privateGpuEffectsDispose() {}
		},
		options
	);
	await processor.applyEffects(request());
	await processor.applyEffects(request());
	assert.equal(detections, 1);
	assert.deepEqual(processor.execution, { backend: 'cpu', fallback: 'Mismatch' });
	processor.dispose();
});

test('execution describes only successful calls and GPU callback failures leave the facade usable', async () => {
	let dispatches = 0;
	const processor = initializeGpuEffects(
		{
			privateGpuEffectsPlanningBytes: () => 1,
			privateGpuEffectsPlan: () => 1,
			privateGpuEffectsDetect: async () => '',
			privateGpuEffectsAdapter: () => 'test adapter',
			privateGpuApplyEffects: async (data, width, height, _effects, _limit, sink) => {
				dispatches++;
				sink.value = { width, height, data: data.slice() };
				return '';
			},
			privateGpuEffectsDispose() {}
		},
		options
	);
	await processor.applyEffects(request());
	const previous = processor.execution;
	await assert.rejects(
		processor.applyEffects(
			request({
				onProgress: () => {
					throw Error('callback');
				}
			})
		),
		diagnostic('callback')
	);
	assert.equal(processor.execution, previous);
	await processor.applyEffects(request());
	assert.equal(dispatches, 3);
	processor.dispose();
});

test('unsupported chains and limits avoid detection/dispatch and preserve CPU chain semantics', async () => {
	for (const planned of [0, options.memoryLimitBytes + 1]) {
		const processor = initializeGpuEffects(
			{
				privateGpuEffectsPlanningBytes: () => 1,
				privateGpuEffectsPlan: () => planned,
				privateGpuEffectsDetect: () => assert.fail('preflight must precede detection'),
				privateGpuApplyEffects: () => assert.fail('preflight must precede dispatch'),
				privateGpuEffectsDispose() {}
			},
			options
		);
		const input = request({
			effects: [
				...effects,
				{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: 0 }
			]
		});
		assert.deepEqual(await processor.applyEffects(input), await expected(input));
		assert.equal(processor.execution.fallback, planned === 0 ? 'UnsupportedChain' : 'MemoryLimit');
		processor.dispose();
	}
});

test('async source and effects are captured once; overlapping calls and disposal reject', async () => {
	let resolve;
	const gate = new Promise((done) => {
		resolve = done;
	});
	const processor = initializeGpuEffects(
		{
			privateGpuEffectsPlanningBytes: () => 1,
			privateGpuEffectsPlan: () => 1,
			privateGpuEffectsDetect: async () => {
				await gate;
				return 'NoDevice';
			},
			privateGpuEffectsDispose() {}
		},
		options
	);
	const input = request({ effects: structuredClone(effects) });
	const wanted = await expected(input);
	const pending = processor.applyEffects(input);
	input.source.data.fill(0);
	input.effects.length = 0;
	await assert.rejects(processor.applyEffects(request()), diagnostic('reentrant-call'));
	assert.throws(() => processor.dispose(), diagnostic('reentrant-call'));
	resolve();
	assert.deepEqual(await pending, wanted);
	processor.dispose();
});

test('no-op, progress failure, invalid request and tight CPU budget preserve lifecycle', async () => {
	const processor = initializeGpuEffects(undefined, options);
	const empty = request({ effects: [] });
	assert.equal(await processor.applyEffects(empty), empty.source);
	await assert.rejects(
		processor.applyEffects(request({ version: 9 })),
		diagnostic('invalid-request')
	);
	await assert.rejects(
		processor.applyEffects(
			request({
				onProgress: () => {
					throw Error('callback');
				}
			})
		),
		diagnostic('callback')
	);
	assert.deepEqual(await processor.applyEffects(request()), await expected(request()));
	processor.dispose();
	const limited = initializeGpuEffects(undefined, { wasm, memoryLimitBytes: 1024 });
	await assert.rejects(limited.applyEffects(request()), diagnostic('memory-limit'));
	limited.dispose();
});
