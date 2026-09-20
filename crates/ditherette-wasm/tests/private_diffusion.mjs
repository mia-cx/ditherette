import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createScalarBindings } from '../dist/scalar/ditherette_wasm.factory.js';

test('private diffusion ABI validates f64 tags before narrowing and keeps failures recoverable', async () => {
	const bindings = createScalarBindings();
	await bindings.default({
		module_or_path: await readFile(
			new URL('../dist/scalar/ditherette_wasm_bg.wasm', import.meta.url)
		)
	});
	assert.equal(bindings.privateInitialize(1_000_000), 0);
	const source = new Uint8Array([100, 100, 100, 255, 100, 100, 100, 255]);
	const controls = [0, 0, 0, 1, 0, 0, 0, 0];
	const invoke = (policy) => {
		const sink = {};
		const status = bindings.privateDitherAndQuantize(
			source,
			2,
			1,
			[0, 0xffffff],
			0,
			0,
			0,
			0,
			2,
			...policy,
			sink
		);
		if (status) assert.equal(sink.value, undefined);
		return { status, output: sink.value };
	};
	try {
		for (const [index, value, path] of [
			[0, 0.5, 31],
			[0, 4, 31],
			[1, 0.5, 32],
			[1, 2, 32],
			[2, 0.5, 33],
			[2, 2, 33],
			[3, Infinity, 26],
			[3, 3.5e38, 26],
			[4, 0.5, 27],
			[5, 1, 27]
		]) {
			const policy = [...controls];
			policy[index] = value;
			assert.equal(invoke(policy).status, 4);
			assert.equal(bindings.privateErrorPath(), path);
			assert.equal(invoke(controls).status, 0);
		}
		for (let kernel = 0; kernel < 4; kernel++) {
			for (let feedback = 0; feedback < 2; feedback++) {
				for (let serpentine = 0; serpentine < 2; serpentine++) {
					assert.equal(invoke([kernel, feedback, serpentine, 1, 1, 1, 5, 10]).status, 0);
				}
			}
		}
		for (const [feedback, path] of [
			[0, 34],
			[1, 35]
		]) {
			assert.equal(invoke([0, feedback, 0, 3.4028234663852886e38, 0, 0, 0, 0]).status, 13);
			assert.equal(bindings.privateErrorPath(), path);
			assert.equal(invoke(controls).status, 0);
		}
	} finally {
		bindings.privateDispose();
	}
});
