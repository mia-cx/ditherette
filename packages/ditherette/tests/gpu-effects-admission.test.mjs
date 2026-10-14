import assert from 'node:assert/strict';
import { cp, mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import test from 'node:test';
import { createDitherette, createGpuEffects } from '../dist/index.js';
import { gpuEffectsDecoderBytes, initializeGpuEffects } from '../dist/gpu-effects.js';
import { validateApplyEffects } from '../dist/validation-effects.js';
import { createScalarBindings } from '../dist/wasm/gpu/ditherette_wasm.factory.js';

const scalar = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const gpu = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/gpu/ditherette_wasm_bg.wasm', import.meta.url))
);
const entries = { gpu: 0, cpu: 0, gpuInstances: 0, cpuInstances: 0 };
const instantiate = WebAssembly.instantiate;
WebAssembly.instantiate = async (module, imports) => {
	const instance = await instantiate(module, imports);
	const exports = { ...instance.exports };
	const backend = module === gpu ? 'gpu' : 'cpu';
	entries[`${backend}Instances`]++;
	const name = backend === 'gpu' ? 'privateGpuEffectsPlan' : 'privateApplyEffects';
	const apply = exports[name];
	exports[name] = (...args) => {
		entries[backend]++;
		return apply(...args);
	};
	// Count entry into the actual Wasm exports, retaining their generated string
	// glue and decoder. No planner/CPU implementation or GPU transport is faked.
	return { instance: { exports }, module };
};
test.after(() => {
	WebAssembly.instantiate = instantiate;
	delete globalThis.navigator;
});

const source = { width: 1, height: 1, data: new Uint8Array([71, 93, 117, 29]) };
const x = { model: 'srgb', channel: 'red' };
const x2 = { model: 'srgb', channel: 'green' };
const grid = {
	columns: Array.from({ length: 48 }, (_, i) => i / 47),
	rows: Array.from({ length: 16 }, (_, i) => i / 15),
	values: Array.from({ length: 16 }, () => Array(48).fill(0))
};
function request(kind, count) {
	return {
		version: 1,
		source,
		effects: Array.from({ length: count }, () =>
			kind === 'curves'
				? {
						effect: 'curves',
						enabled: true,
						curves: Array(16).fill({
							kind: 'adjust',
							x,
							x2,
							y: { model: 'srgb', channel: 'blue' },
							grid
						})
					}
				: { effect: 'exposure', enabled: true, stops: 0.5, mask: Array(4).fill({ x, x2, grid }) }
		)
	};
}
const control = {
	version: 1,
	source,
	effects: [{ effect: 'exposure', enabled: true, stops: 0.5 }]
};
const options = { wasm: scalar, gpuWasm: gpu, memoryLimitBytes: 2 * 1024 ** 2 };
const capability = () => {
	globalThis.navigator = { gpu: { requestAdapter: () => assert.fail('no hardware request') } };
};
async function rejectedWithoutEntry(processor, input) {
	const before = { ...entries };
	await assert.rejects(processor.applyEffects(input), {
		code: 'memory-limit',
		path: 'memoryLimitBytes'
	});
	assert.deepEqual(entries, before, 'no GPU planner, CPU decoder, or scalar instance allocation');
}

test('the common reserve rejects one byte below admission, including 600-byte scalar-only chains', async () => {
	for (const capabilityPresent of [false, true]) {
		delete globalThis.navigator;
		if (capabilityPresent) capability();
		for (const kind of ['curves', 'masks']) {
			const input = request(kind, 8);
			const inputBytes = validateApplyEffects(input).effects.json.length;
			const bound = gpuEffectsDecoderBytes(inputBytes) + source.data.length;
			const processor = await createGpuEffects({ ...options, memoryLimitBytes: bound - 1 });
			await rejectedWithoutEntry(processor, input);
			processor.dispose();
			console.log(
				JSON.stringify({
					capabilityPresent,
					kind,
					inputBytes,
					bound,
					limit: bound - 1,
					plannerEntries: 0,
					scalarEntries: 0
				})
			);
		}
		const input = { ...control, effects: Array(64).fill(control.effects[0]) };
		assert.equal(validateApplyEffects(input).effects.json.length, 3137);
		const processor = await createGpuEffects({ ...options, memoryLimitBytes: 600 });
		await rejectedWithoutEntry(processor, input);
		processor.dispose();
	}
});

test('large valid curves and masks reject before both real string ABIs on every facade route', async (t) => {
	const directory = await mkdtemp(join(process.env.TMPDIR ?? tmpdir(), 'gpu-admission-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	await cp(new URL('../dist/', import.meta.url), directory, {
		recursive: true,
		filter: (path) => !path.includes('/wasm/gpu')
	});
	const missing = await import(pathToFileURL(join(directory, 'gpu-effects.js')).href);
	for (const route of ['no-capability', 'missing-assets', 'gpu-artifact', 'retired']) {
		delete globalThis.navigator;
		let processor;
		if (route === 'no-capability') processor = await createGpuEffects(options);
		else {
			capability();
			processor = await (route === 'missing-assets' ? missing.createGpuEffects : createGpuEffects)(
				options
			);
		}
		if (route === 'retired') {
			const input = request('masks', 1);
			input.effects[0].mask = [
				{
					x,
					points: [
						[0, 0],
						[1, 1]
					]
				}
			];
			await processor.applyEffects(input);
			assert.equal(processor.execution.fallback, 'UnsupportedChain');
		}
		for (const kind of ['curves', 'masks']) {
			for (const count of [8, 64]) await rejectedWithoutEntry(processor, request(kind, count));
		}
		const cpu = await createDitherette({ wasm: scalar });
		assert.deepEqual(await processor.applyEffects(control), cpu.applyEffects(control));
		assert.equal(processor.execution.backend, 'cpu');
		cpu.dispose();
		processor.dispose();
	}
});

test('in-budget real unsupported decoding succeeds, then later admission and fresh-facade recovery stay bounded', async () => {
	capability();
	for (const kind of ['curves', 'masks']) {
		const input = request(kind, 8);
		const artifact = createScalarBindings();
		await artifact.default({ module_or_path: gpu });
		const inputBytes = validateApplyEffects(input).effects.json.length;
		const limit = artifact.privateGpuEffectsPlanningBytes(inputBytes) + source.data.length;
		const processor = initializeGpuEffects(artifact, { wasm: scalar, memoryLimitBytes: limit });
		const before = { ...entries };
		const cpu = await createDitherette({ wasm: scalar });
		assert.deepEqual(await processor.applyEffects(input), cpu.applyEffects(input));
		assert.equal(entries.gpu - before.gpu, 1);
		assert.equal(entries.cpu - before.cpu, 2);
		assert.equal(processor.execution.fallback, 'UnsupportedChain');
		await rejectedWithoutEntry(processor, request(kind, 64));
		assert.deepEqual(await processor.applyEffects(control), cpu.applyEffects(control));
		assert.equal(processor.execution.fallback, 'UnsupportedChain');
		processor.dispose();
		cpu.dispose();
		const fresh = await createGpuEffects({ ...options, memoryLimitBytes: limit });
		const newEntries = { ...entries };
		assert.equal((await fresh.applyEffects(input)).data[3], 29);
		assert.equal(entries.gpu - newEntries.gpu, 1, 'fresh facade can enter GPU planning again');
		assert.equal(entries.cpu - newEntries.cpu, 1);
		fresh.dispose();
		console.log(
			JSON.stringify({
				kind,
				inputBytes,
				admissionBytes: limit,
				plannerEntries: 1,
				scalarEntries: 1,
				laterOverBudgetEntries: 0
			})
		);
	}
});

test('large disabled and empty chains keep source identity without decoder admission or Wasm entry', async () => {
	const processor = await createGpuEffects({ ...options, memoryLimitBytes: 1 });
	const before = { ...entries };
	const input = request('curves', 64);
	for (const step of input.effects) step.enabled = false;
	assert.equal(await processor.applyEffects(input), source);
	assert.equal(await processor.applyEffects({ ...input, effects: [] }), source);
	assert.deepEqual(entries, before);
	processor.dispose();
});
