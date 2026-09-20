import assert from 'node:assert/strict';
import test from 'node:test';
import { snapshotInput } from '../../../crates/ditherette-wasm/src/wasm/copy_helpers.js';

test('exact source comparison covers aligned words, offset views, tails, and every mutated byte', () => {
	for (const size of [4, 9, 15, 16]) {
		for (const offset of [0, 1, 4]) {
			const source = new Uint8Array(size + offset).subarray(offset);
			source.fill(23);
			const destination = new Uint8Array(size);
			assert.equal(snapshotInput(destination, source, false), false);
			assert.equal(snapshotInput(destination, source, true), true);
			for (let index = 0; index < size; index++) {
				source[index] ^= 1;
				assert.equal(snapshotInput(destination, source, true), false);
				assert.deepEqual(destination, source);
				assert.equal(snapshotInput(destination, source, true), true);
			}
		}
	}
});

test('comparison ignores shadowed view properties and rejects detached storage', () => {
	const source = new Uint8Array([1, 2, 3, 4]);
	const destination = source.slice();
	for (const key of ['buffer', 'byteOffset', 'length']) {
		Object.defineProperty(source, key, {
			get() {
				throw new Error('shadowed getter');
			}
		});
	}
	assert.equal(snapshotInput(destination, source, true), true);
	source[3] = 9;
	assert.equal(snapshotInput(destination, source, true), false);
	assert.equal(destination[3], 9);
	const detached = new Uint8Array(4);
	structuredClone(detached.buffer, { transfer: [detached.buffer] });
	assert.throws(() => snapshotInput(destination, detached, true), TypeError);
});
