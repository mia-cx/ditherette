import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createScalarBindings } from '../dist/wasm/gpu/ditherette_wasm.factory.js';
import { gpuEffectsDecoderBytes, initializeGpuEffects } from '../dist/gpu-effects.js';
import { createDitherette } from '../dist/index.js';
import { initializeGpuDither } from '../dist/gpu-dither.js';

const scalar = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const gpu = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/gpu/ditherette_wasm_bg.wasm', import.meta.url))
);
const bindings = createScalarBindings();
await bindings.default({ module_or_path: gpu });
const effects = [{ effect: 'brightness-contrast', enabled: true, brightness: 0.1, contrast: 0.3 }];
const source = {
	width: 37,
	height: 19,
	data: Uint8Array.from({ length: 37 * 19 * 4 }, (_, i) => i & 255)
};
test('ordinary Wasm has no GPU exports; the opt-in ABI plans exact enabled-chain eligibility', () => {
	assert.ok(!WebAssembly.Module.exports(scalar).some(({ name }) => name.startsWith('privateGpu')));
	assert.ok(WebAssembly.Module.exports(gpu).some(({ name }) => name === 'privateGpuApplyEffects'));
	for (const bytes of [0, 1, 3137, 2_978_689])
		assert.equal(bindings.privateGpuEffectsPlanningBytes(bytes), gpuEffectsDecoderBytes(bytes));
	const plan = (steps) =>
		bindings.privateGpuEffectsPlan(JSON.stringify(steps), source.width, source.height);
	assert.ok(plan(effects) > source.data.length * 7);
	const mixed = { effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: 0 };
	assert.equal(plan([...effects, mixed]), 0);
	assert.ok(plan([...effects, { ...mixed, enabled: false }]) > 0);
	assert.equal(
		plan([
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
		]),
		0
	);
});

test('real Wasm memory plan routes a package operation to CPU before GPU detection', async () => {
	// Use enough pixels that GPU staging/readback exceeds the scalar processor's
	// larger fixed analysis/cache bookkeeping, while CPU fallback still fits.
	const source = { width: 512, height: 512, data: new Uint8Array(512 * 512 * 4) };
	const request = { version: 1, source, effects };
	const limit =
		bindings.privateGpuEffectsPlan(JSON.stringify(effects), source.width, source.height) - 1;
	const processor = initializeGpuEffects(
		{
			...bindings,
			privateGpuEffectsDetect: () => assert.fail('budget rejection precedes detection')
		},
		{ wasm: scalar, memoryLimitBytes: limit }
	);
	const cpu = await createDitherette({ wasm: scalar });
	try {
		assert.deepEqual(await processor.applyEffects(request), cpu.applyEffects(request));
		assert.equal(processor.execution.fallback, 'MemoryLimit');
	} finally {
		processor.dispose();
		cpu.dispose();
	}
});

test('indexed GPU admission falls back before device detection using the actual numeric ABI', async () => {
	const source = { width: 512, height: 512, data: new Uint8Array(512 * 512 * 4).fill(127) };
	const request = {
		version: 1,
		source,
		palette: [
			{ kind: 'color', rgb: [0, 0, 0] },
			{ kind: 'color', rgb: [255, 255, 255] }
		],
		alpha: { mode: 'preserve', threshold: 0 },
		matching: 'oklab-euclidean',
		dither: { family: 'none' }
	};
	for (const dither of [
		{ family: 'none' },
		{ family: 'yliluoma', size: '16', placement: { mode: 'everywhere' } },
		{
			family: 'diffusion',
			kernel: 'sierra',
			feedback: 'matching',
			serpentine: true,
			strength: 1,
			placement: { mode: 'everywhere' }
		}
	]) {
		request.dither = dither;
		if (dither.family === 'yliluoma') {
			request.palette.push(
				...Array.from({ length: 30 }, (_, i) => ({ kind: 'color', rgb: [i * 7, i * 3, i * 5] }))
			);
		}
		const limit =
			(dither.family === 'yliluoma'
				? bindings.privateGpuYliluomaPlan(
						source.width,
						source.height,
						request.palette.length,
						Number(dither.size)
					)
				: dither.family === 'diffusion'
					? bindings.privateGpuDiffusionPlan(source.width, source.height)
					: bindings.privateGpuDitherPlan(source.width, source.height)) - 1;
		const processor = initializeGpuDither(
			{
				...bindings,
				privateGpuDitherDetect: () => assert.fail('numeric admission must precede device detection')
			},
			{ wasm: scalar, memoryLimitBytes: limit, diffusionBackend: 'gpu' }
		);
		const cpu = await createDitherette({ wasm: scalar });
		try {
			assert.deepEqual(await processor.ditherAndQuantize(request), cpu.ditherAndQuantize(request));
			assert.equal(processor.execution.fallback, 'MemoryLimit');
		} finally {
			processor.dispose();
			cpu.dispose();
		}
	}
});

test('600-byte admission rejects 64 real exposure steps before the Wasm string planner', async () => {
	for (const mixed of [false, true]) {
		const artifact = createScalarBindings();
		await artifact.default({ module_or_path: gpu });
		let plannerEntries = 0;
		let numericAdmissions = 0;
		const bounded = initializeGpuEffects(
			{
				...artifact,
				privateGpuEffectsPlanningBytes(bytes) {
					numericAdmissions++;
					return artifact.privateGpuEffectsPlanningBytes(bytes);
				},
				privateGpuEffectsPlan(...args) {
					plannerEntries++;
					return artifact.privateGpuEffectsPlan(...args);
				}
			},
			{ wasm: scalar, memoryLimitBytes: 600 }
		);
		const effects = Array.from({ length: 64 }, () => ({
			effect: 'exposure',
			enabled: true,
			stops: 0.5
		}));
		if (mixed)
			effects[63] = {
				effect: 'hue-saturation',
				enabled: true,
				hue: 30,
				saturation: 0.2,
				lightness: 0
			};
		await assert.rejects(
			bounded.applyEffects({
				version: 1,
				source: { width: 1, height: 1, data: new Uint8Array(4) },
				effects
			}),
			{ code: 'memory-limit', path: 'memoryLimitBytes' }
		);
		assert.equal(numericAdmissions, 0, 'admission metadata does not require GPU bindings');
		assert.equal(plannerEntries, 0);
		bounded.dispose();
	}
});
