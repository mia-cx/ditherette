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
				// Equal snapshots skip input copying. Change an ignored transparent RGB byte
				// so this fixture still reaches its original three caught-copy positions.
				source[0] = 1;
				try {
					failed = invoke();
				} finally {
					Uint8Array.prototype.set = set;
					source[0] = 0;
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

const palette = [0, 0xffffff, 0xffffff, 0x1000000];
const modes = [
	[0, 0, 0, 0, 0, 0, 0, 0, 0],
	[1, 2, 0, 2, 0.7, 1, 1, 0.3, 0.2],
	[2, 0, 0, 1, 0.7, 0, 0, 0, 0],
	[2, 0, 1, 1, 1, 0, 0, 0, 0],
	[3, 0, 4, 0, 0, 0, 0, 0, 0]
];

async function fresh(limit = 1 << 20) {
	const bindings = createScalarBindings();
	const raw = await bindings.default({ module_or_path: await readFile(new URL('../dist/scalar/ditherette_wasm_bg.wasm', import.meta.url)) });
	assert.equal(bindings.privateInitialize(limit), 0);
	return { bindings, raw };
}

function process(bindings, input, source, output, mode = modes[0], sink = {}) {
	const status = bindings.privateProcess(input, ...source, palette, 1, ...output, 0, 4, 0, 0, 0, 128, 0, ...mode, sink);
	return { status, value: sink.value };
}

function staged(bindings, input, source, output, mode) {
	const resized = {};
	assert.equal(bindings.privateResize(input, ...source, ...output, 0, 4, 0, resized), 0);
	const indexed = {};
	assert.equal(bindings.privateDitherAndQuantize(resized.value.data, ...output, palette, 0, 0, 128, 0, ...mode, indexed), 0);
	return indexed.value;
}

test('sparse process matches staged modes on offset views without full input or intermediate JS copies', async () => {
	const { bindings, raw } = await fresh();
	const reference = await fresh();
	try {
		for (const [source, output] of [[[43, 37], [7, 5]], [[256, 256], [64, 64]], [[64, 64], [32, 32]]]) {
			for (const offset of [0, 1, 4]) {
				const backing = Uint8Array.from({ length: source[0] * source[1] * 4 + offset }, (_, i) => (i * 73 + Math.floor(i / 251)) & 255);
				const input = backing.subarray(offset);
				const original = input.slice();
				for (const mode of modes) {
					const expected = staged(reference.bindings, input, source, output, mode);
					const set = Uint8Array.prototype.set;
					let copies = 0;
					Uint8Array.prototype.set = function (...args) {
						assert.notEqual(this.buffer, raw.memory.buffer, 'fused sparse input uses gather');
						copies++;
						return Reflect.apply(set, this, args);
					};
					let result;
					try { result = process(bindings, input, source, output, mode); }
					finally { Uint8Array.prototype.set = set; }
					assert.equal(result.status, 0);
					assert.deepEqual(result.value, expected);
					assert.equal(copies, 2, 'only durable indices and palette leave Wasm');
				}
				assert.deepEqual(input, original);
			}
		}
	} finally {
		bindings.privateDispose();
		reference.bindings.privateDispose();
	}
});

test('sparse process stays fresh across source mutations, geometry changes, and full-source calls', async () => {
	const { bindings, raw } = await fresh();
	const reference = await fresh();
	const input = Uint8Array.from({ length: 64 * 64 * 4 }, (_, i) => i & 255);
	const durable = process(bindings, input, [64, 64], [4, 4]).value;
	const saved = structuredClone(durable);
	for (const output of [[48, 48], [4, 4], [32, 32], [3, 5], [4, 4], [48, 48], [4, 4]]) {
		input.fill(output[0] === 4 ? 255 : 0);
		for (const mode of modes) {
			const result = process(bindings, input, [64, 64], output, mode);
			assert.equal(result.status, 0);
			assert.deepEqual(result.value, staged(reference.bindings, input, [64, 64], output, mode));
		}
	}
	raw.memory.grow(1);
	bindings.privateDispose();
	reference.bindings.privateDispose();
	assert.deepEqual(durable, saved);
	assert.equal(process(bindings, input, [64, 64], [4, 4]).status, 10);
});

test('sparse process catches gather, completion, and progress failures and detached input', async () => {
	const { bindings, raw } = await fresh();
	let input = new Uint8Array(64 * 64 * 4).fill(255);
	const invoke = sink => process(bindings, input, [64, 64], [4, 4], modes[1], sink);
	const expected = invoke().value;
	const table = Object.values(raw).find(value => value instanceof WebAssembly.Table);
	const state = () => [table.length, Array.from({ length: table.length }, (_, i) => table.get(i)).filter(value => value !== null).length, raw.memory.buffer.byteLength];
	const before = state();
	for (let i = 0; i < 32; i++) {
		const get = DataView.prototype.getUint32;
		DataView.prototype.getUint32 = function () { throw new Error('gather failed'); };
		let failed;
		try { failed = invoke(); }
		finally { DataView.prototype.getUint32 = get; }
		assert.deepEqual(failed, { status: 9, value: undefined });
		assert.equal(bindings.privateErrorPath(), 4);
		assert.deepEqual(invoke().value, expected);
		const sink = { onProgress(event) {
			assert.equal(invoke().status, 11);
			assert.equal(bindings.privateDispose(), 11);
			if (event.stage === 'complete') throw new Error('completion failed');
		} };
		assert.deepEqual(invoke(sink), { status: 12, value: undefined });
		assert.deepEqual(invoke().value, expected);
		assert.deepEqual(invoke(Object.freeze({})), { status: 9, value: undefined });
	}
	assert.deepEqual(state(), before);
	const sink = { onProgress(event) {
		if (event.stage === 'prepare') structuredClone(input.buffer, { transfer: [input.buffer] });
	} };
	assert.deepEqual(invoke(sink), { status: 9, value: undefined });
	assert.equal(bindings.privateErrorPath(), 4);
	assert.equal(invoke().status, 2);
	input = new Uint8Array(64 * 64 * 4).fill(255);
	assert.deepEqual(invoke().value, expected);
	bindings.privateDispose();
});
