import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createScalarBindings } from '../dist/wasm/gpu/ditherette_wasm.factory.js';
import { initializeGpuEffects } from '../dist/gpu-effects.js';
import { createDitherette } from '../dist/index.js';

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
const request = { version: 1, source, effects };

test('ordinary Wasm has no GPU exports; the opt-in ABI plans exact enabled-chain eligibility', () => {
	assert.ok(!WebAssembly.Module.exports(scalar).some(({ name }) => name.startsWith('privateGpu')));
	assert.ok(WebAssembly.Module.exports(gpu).some(({ name }) => name === 'privateGpuApplyEffects'));
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
