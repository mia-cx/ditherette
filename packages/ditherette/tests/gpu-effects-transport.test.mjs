import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { setImmediate } from 'node:timers/promises';
import test from 'node:test';
import { createDitherette, createGpuEffects } from '../dist/index.js';
import { createScalarBindings } from '../dist/wasm/gpu/ditherette_wasm.factory.js';
import { installGpuTransport } from './gpu-transport.mjs';

// Only WebGPU transport is synthetic. The public factory, Wasm planner/tables,
// upload, mapping, exception boundary and scalar fallback are the built artifacts.
const scalar = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const gpu = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/gpu/ditherette_wasm_bg.wasm', import.meta.url))
);
const { resources, observations } = installGpuTransport();
const gpuMemories = [];
let retiredMemory;
let failResultCopy = false;
const instantiate = WebAssembly.instantiate;
WebAssembly.instantiate = async (module, imports) => {
	if (module === scalar && retiredMemory) {
		assert.equal(
			observations.destroyed,
			observations.devices,
			'device released before scalar allocation'
		);
		assert.equal(resources.size, 0, 'GPU buffers released before scalar allocation');
		// Compiled modules are caller-owned inputs. Keep no caller-owned bindings or
		// exports: only WeakRefs can prove the library retired the owning instance.
		for (let attempt = 0; attempt < 10; attempt++) {
			await setImmediate();
			global.gc();
			if (!retiredMemory.deref()) break;
		}
		assert.equal(
			retiredMemory.deref(),
			undefined,
			'GPU instance unreachable before scalar allocation'
		);
		retiredMemory = undefined;
	}
	if (module === gpu) {
		for (const namespace of Object.values(imports)) {
			for (const [name, fn] of Object.entries(namespace)) {
				if (!name.includes('completeResult')) continue;
				namespace[name] = (...args) => {
					if (failResultCopy) {
						failResultCopy = false;
						// Throw from inside completeResult, preserving wasm-bindgen's
						// generated catch import. Throwing outside fn would bypass it.
						args[4] = {
							set value(_image) {
								throw new DOMException(
									'Injected synchronous result-copy failure.',
									'SecurityError'
								);
							}
						};
					}
					return fn(...args);
				};
			}
		}
	}
	const result = await instantiate(module, imports);
	if (module === gpu) gpuMemories.push(new WeakRef((result.instance ?? result).exports.memory));
	return result;
};

const options = { wasm: scalar, gpuWasm: gpu, memoryLimitBytes: 32 * 1024 ** 2 };
const source = {
	width: 37,
	height: 19,
	data: Uint8Array.from({ length: 37 * 19 * 4 }, (_, i) => (i * 61 + (i >> 2)) & 255)
};
const effects = [{ effect: 'exposure', enabled: true, stops: 0.5 }];
const request = { version: 1, source, effects };
const unsupported = {
	...request,
	context: {
		palette: [
			{ kind: 'color', rgb: [0, 0, 0] },
			{ kind: 'color', rgb: [255, 255, 255] }
		]
	},
	effects: [
		...effects,
		{
			effect: 'palette-fit',
			enabled: true,
			look: 'natural',
			space: 'oklab',
			strength: 1,
			curves: null
		}
	]
};

test(
	'public packed upload settles without external-image calls; fallback retires its artifact before scalar allocation',
	{ timeout: 30000 },
	async (t) => {
		assert.equal(typeof global.gc, 'function', 'run the real lifetime proof with node --expose-gc');
		const cpu = await createDitherette({ wasm: scalar });
		const wanted = cpu.applyEffects(request);
		const mixed = cpu.applyEffects(unsupported);
		cpu.dispose();
		const processor = await createGpuEffects(options);
		const output = await processor.applyEffects(request);
		assert.deepEqual(output, wanted);
		assert.equal(processor.execution.backend, 'gpu');
		assert.match(processor.execution.adapter, /BrowserWebGpu/);
		for (let i = 0; i < source.data.length; i += 4)
			assert.equal(output.data[i + 3], source.data[i + 3]);
		for (const steps of [[], effects.map((step) => ({ ...step, enabled: false }))]) {
			assert.equal(await processor.applyEffects({ ...request, effects: steps }), source);
		}
		const eligibleHighWaterBytes = gpuMemories.at(-1).deref().buffer.byteLength;
		for (let i = 0; i < 32; i++) assert.deepEqual(await processor.applyEffects(request), wanted);
		assert.equal(gpuMemories.at(-1).deref().buffer.byteLength, eligibleHighWaterBytes);
		assert.equal(observations.devices, 1, 'eligible calls reuse the verified device');
		assert.equal(observations.handlers, 1, 'wgpu registers one uncaptured-error callback');
		assert.equal(observations.lossHandlers, 1, 'wgpu registers one device-loss callback');
		assert.equal(observations.destroyed, 0, 'no-op calls preserve the device');
		retiredMemory = gpuMemories.at(-1);
		assert.deepEqual(await processor.applyEffects(unsupported), mixed);
		for (let i = 0; i < 32; i++) {
			const input = i % 2 ? unsupported : request;
			assert.deepEqual(await processor.applyEffects(input), i % 2 ? mixed : wanted);
			assert.deepEqual(processor.execution, { backend: 'cpu', fallback: 'UnsupportedChain' });
		}
		assert.equal(observations.devices, 1, 'retired facade never re-detects');
		assert.equal(observations.lossHandlers, 1);
		assert.equal(
			observations.handlers,
			1,
			'repeated fallback cannot accumulate callback ownership'
		);
		processor.dispose();
		processor.dispose();
		assert.deepEqual(output, wanted, 'earlier output survives retirement and disposal');
		const fresh = await createGpuEffects(options);
		assert.deepEqual(await fresh.applyEffects(request), wanted);
		assert.equal(fresh.execution.backend, 'gpu', 'a fresh facade supports GPU recovery');
		fresh.dispose();
		assert.equal(observations.devices, 2);
		assert.equal(observations.handlers, 2);
		assert.equal(observations.lossHandlers, 2);
		assert.equal(observations.destroyed, 2);
		assert.equal(
			observations.externalUploads,
			0,
			'the unsafe DOM upload is unreachable through this API'
		);
		assert.equal(resources.size, 0);
		t.diagnostic(
			JSON.stringify({
				...observations,
				repeatedEligibleCalls: 32,
				alternatingFallbackCalls: 32,
				eligibleHighWaterBytes,
				rgbaDifference: 0
			})
		);
	}
);

test(
	'a synchronous exception in the actual packed Wasm result bridge settles and releases before CPU fallback',
	{ timeout: 30000 },
	async () => {
		const processor = await createGpuEffects(options);
		const cpu = await createDitherette({ wasm: scalar });
		const wanted = cpu.applyEffects(request);
		cpu.dispose();
		failResultCopy = true;
		retiredMemory = gpuMemories.at(-1);
		assert.deepEqual(await processor.applyEffects(request), wanted);
		assert.equal(
			failResultCopy,
			false,
			'the synchronous exception crossed the actual Wasm catch import'
		);
		assert.equal(processor.execution.backend, 'cpu');
		assert.equal(processor.execution.fallback, 'Failed');
		assert.equal(resources.size, 0);
		processor.dispose();
		await assert.rejects(processor.applyEffects(request), { code: 'disposed' });
	}
);

test('private real Wasm ABI never requests another device from a released owning artifact', async () => {
	// These explicitly caller-owned bindings pin their instance. The library-owned
	// lifetime proof above deliberately retains no such references.
	const bindings = createScalarBindings();
	await bindings.default({ module_or_path: gpu });
	assert.equal(await bindings.privateGpuEffectsDetect(options.memoryLimitBytes), '');
	bindings.privateGpuEffectsDispose();
	const devices = observations.devices;
	const handlers = observations.handlers;
	const lossHandlers = observations.lossHandlers;
	for (let i = 0; i < 8; i++) {
		assert.equal(
			await bindings.privateGpuEffectsDetect(options.memoryLimitBytes),
			'ArtifactRetired'
		);
	}
	assert.equal(observations.devices, devices);
	assert.equal(observations.handlers, handlers);
	assert.equal(observations.lossHandlers, lossHandlers);
	assert.equal(resources.size, 0);
});

test(
	'large retained GPU heap is retired at the real budget before a larger scalar fallback',
	{ timeout: 30000 },
	async (t) => {
		const source = { width: 1024, height: 1024, data: new Uint8Array(1024 * 1024 * 4).fill(71) };
		// This diagnostic planner instance belongs to the caller, not the facade.
		const planner = createScalarBindings();
		await planner.default({ module_or_path: gpu });
		const memoryLimitBytes = planner.privateGpuEffectsPlan(
			JSON.stringify(effects),
			source.width,
			source.height
		);
		const processor = await createGpuEffects({ ...options, memoryLimitBytes });
		await processor.applyEffects({ version: 1, source, effects });
		assert.equal(processor.execution.backend, 'gpu');
		const memory = gpuMemories.at(-1);
		const gpuHighWaterBytes = memory.deref().buffer.byteLength;
		const fallbackSource = {
			width: 1400,
			height: 1400,
			data: new Uint8Array(1400 * 1400 * 4).fill(71)
		};
		const fallback = {
			version: 1,
			source: fallbackSource,
			effects: [
				{
					...effects[0],
					mask: [
						{
							x: { model: 'srgb', channel: 'red' },
							points: [
								[0, 0],
								[1, 1]
							]
						}
					]
				}
			]
		};
		retiredMemory = memory;
		const output = await processor.applyEffects(fallback);
		assert.equal(processor.execution.backend, 'cpu');
		assert.equal(processor.execution.fallback, 'MemoryLimit');
		const cpu = await createDitherette({ wasm: scalar });
		assert.deepEqual(output, cpu.applyEffects(fallback));
		cpu.dispose();
		processor.dispose();
		assert.equal(memory.deref(), undefined);
		assert.equal(resources.size, 0);
		t.diagnostic(
			JSON.stringify({
				memoryLimitBytes,
				gpuHighWaterBytes,
				scalarSourceSnapshotBytes: fallbackSource.data.length,
				ownedGpuInstanceAtScalarAllocation: false
			})
		);
	}
);
