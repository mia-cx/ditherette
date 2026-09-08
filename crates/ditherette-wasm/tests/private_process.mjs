import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createScalarBindings } from '../dist/scalar/ditherette_wasm.factory.js';

test('private process keeps borrowed input and publishes only the composed indexed result', async () => {
	const bindings = createScalarBindings();
	await bindings.default({
		module_or_path: await readFile(
			new URL('../dist/scalar/ditherette_wasm_bg.wasm', import.meta.url)
		)
	});
	assert.equal(bindings.privateInitialize(1_000_000), 0);
	try {
		const source = new Uint8Array([0, 0, 0, 0, 255, 255, 255, 255]);
		const sink = {};
		assert.equal(
			bindings.privateProcess(
				source,
				2,
				1,
				[0, 0xffffff, 0x1000000],
				1,
				4,
				1,
				0,
				4,
				0,
				0,
				0,
				128,
				0,
				0,
				0,
				0,
				0,
				0,
				0,
				0,
				0,
				0,
				sink
			),
			0
		);
		assert.equal(sink.value.width, 4);
		assert.deepEqual([...sink.value.indices], [2, 2, 1, 1]);
		assert.equal(sink.value.palette.transparentIndex, 2);
	} finally {
		bindings.privateDispose();
	}
});

test('private process validates every numeric group and recovers without retained handles', async () => {
	const bindings = createScalarBindings();
	const raw = await bindings.default({
		module_or_path: await readFile(
			new URL('../dist/scalar/ditherette_wasm_bg.wasm', import.meta.url)
		)
	});
	assert.equal(bindings.privateInitialize(1_000_000), 0);
	const source = new Uint8Array([0, 0, 0, 0, 255, 255, 255, 255]);
	const controls = [1, 4, 1, 0, 4, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
	const invoke = (settings = controls, sink = {}) => ({
		status: bindings.privateProcess(source, 2, 1, [0, 0xffffff, 0x1000000], ...settings, sink),
		output: sink.value
	});
	try {
		for (const [index, value, code, path] of [
			[0, 1.5, 1, 37],
			[0, NaN, 1, 37],
			[1, 1.5, 4, 6],
			[2, Infinity, 4, 7],
			[3, 6.5, 4, 12],
			[4, NaN, 4, 9],
			[5, 1, 4, 12],
			[6, 1.5, 5, 16],
			[7, 3, 4, 14],
			[8, NaN, 4, 15],
			[10, 5, 5, 25],
			[11, 1, 4, 25]
		]) {
			const settings = [...controls];
			settings[index] = value;
			const result = invoke(settings);
			assert.equal(result.status, code, `control ${index}`);
			assert.equal(bindings.privateErrorPath(), path);
			assert.equal(result.output, undefined);
		}
		for (const family of [1, 2, 3]) {
			const settings = [...controls];
			settings[10] = family;
			if (family === 1) settings[11] = 2;
			if (family === 3) settings[12] = 4;
			assert.equal(invoke(settings).status, 0);
		}
		const expected = invoke().output;
		const table = Object.values(raw).find((entry) => entry instanceof WebAssembly.Table);
		const state = () => ({
			slots: table.length,
			live: Array.from({ length: table.length }, (_, i) => table.get(i)).filter(
				(entry) => entry !== null
			).length,
			memory: raw.memory.buffer.byteLength
		});
		const before = state();
		for (let cycle = 0; cycle < 64; cycle++) {
			for (const failAt of [1, 2, 3]) {
				const set = Uint8Array.prototype.set;
				let copies = 0;
				Uint8Array.prototype.set = function (...args) {
					if (++copies === failAt) {
						assert.equal(invoke().status, 11);
						assert.equal(bindings.privateDispose(), 11);
						throw new RangeError('process copy');
					}
					return Reflect.apply(set, this, args);
				};
				let failed;
				try {
					failed = invoke();
				} finally {
					Uint8Array.prototype.set = set;
				}
				assert.equal(failed.status, 9);
				assert.equal(failed.output, undefined);
				assert.equal(bindings.privateErrorPath(), failAt === 1 ? 4 : 8);
				assert.deepEqual(invoke().output, expected);
			}
			assert.equal(invoke(controls, Object.freeze({})).status, 9);
		}
		assert.deepEqual(state(), before);
	} finally {
		bindings.privateDispose();
	}
	assert.equal(invoke().status, 10);
});
