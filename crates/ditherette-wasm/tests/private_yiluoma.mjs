import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createScalarBindings } from '../dist/scalar/ditherette_wasm.factory.js';

test('private Yliluoma preserves strict borrowed controls, caught copies, reentry, and bounded handles', async () => {
	const bindings = createScalarBindings();
	const raw = await bindings.default({
		module_or_path: await WebAssembly.compile(
			await readFile(new URL('../dist/scalar/ditherette_wasm_bg.wasm', import.meta.url))
		)
	});
	assert.equal(bindings.privateInitialize(1_000_000), 0);
	const bytes = new Uint8Array([
		64, 64, 64, 255, 64, 64, 64, 255, 64, 64, 64, 255, 64, 64, 64, 255
	]);
	const controls = [0, 2, 0, 0, 1, 1, 100, 0];
	const invoke = (policy = controls, sink = {}) => {
		const status = bindings.privateDitherAndQuantize(
			bytes,
			2,
			2,
			[0, 0x808080, 0x404040],
			0,
			0,
			0,
			0,
			3,
			...policy,
			sink
		);
		return { status, output: sink.value };
	};
	try {
		assert.deepEqual([...invoke().output.indices], [1, 0, 0, 1]);
		for (const [slot, value, path] of [
			[0, 1, 25],
			[1, 3, 36],
			[1, 2.5, 36],
			[1, NaN, 36],
			[1, Infinity, 36],
			[2, 1, 25],
			[3, 1, 25],
			[4, 2, 27],
			[5, 0, 28],
			[5, 1.5, 28],
			[6, Infinity, 29],
			[7, 3.5e38, 30]
		]) {
			const policy = [...controls];
			policy[slot] = value;
			assert.equal(invoke(policy).status, 4);
			assert.equal(bindings.privateErrorPath(), path);
		}
		const table = Object.values(raw).find((value) => value instanceof WebAssembly.Table);
		const state = () => ({
			slots: table.length,
			live: Array.from({ length: table.length }, (_, i) => table.get(i)).filter(
				(value) => value !== null
			).length,
			bytes: raw.memory.buffer.byteLength
		});
		const before = state();
		for (let cycle = 0; cycle < 64; cycle++) {
			for (let failAt = 1; failAt <= 3; failAt++) {
				const set = Uint8Array.prototype.set;
				let copies = 0;
				Uint8Array.prototype.set = function (...args) {
					if (++copies === failAt) {
						assert.equal(invoke().status, 11);
						assert.equal(bindings.privateDispose(), 11);
						throw new RangeError('copy');
					}
					return Reflect.apply(set, this, args);
				};
				try {
					assert.equal(invoke().status, 9);
				} finally {
					Uint8Array.prototype.set = set;
				}
				assert.equal(bindings.privateErrorPath(), failAt === 1 ? 4 : 8);
				assert.deepEqual([...invoke().output.indices], [1, 0, 0, 1]);
			}
			assert.equal(invoke(controls, Object.freeze({})).status, 9);
		}
		assert.deepEqual(state(), before);
	} finally {
		bindings.privateDispose();
	}
	assert.equal(invoke().status, 10);
});
