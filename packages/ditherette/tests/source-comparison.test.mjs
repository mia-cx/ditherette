import assert from 'node:assert/strict';
import test from 'node:test';
import { completeSparseResult, gatherInput, snapshotInput } from '../../../crates/ditherette-wasm/src/wasm/copy_helpers.js';

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

test('direct sparse output repeats rows only for aligned ordinary scalar buffers', (t) => {
	const copyWithin = Uint32Array.prototype.copyWithin;
	let copies = 0;
	t.mock.method(Uint32Array.prototype, 'copyWithin', function (...args) {
		copies++;
		return Reflect.apply(copyWithin, this, args);
	});
	for (const sourceOffset of [0, 1, 4]) for (const sharedSource of [false, true]) {
		const BufferType = sharedSource ? SharedArrayBuffer : ArrayBuffer;
		const source = new Uint8Array(new BufferType(48 + sourceOffset), sourceOffset, 48);
		source.set(Uint8Array.from({ length: 48 }, (_, i) => (i * 73) & 255));
		for (const sharedOffsets of [false, true]) for (const rowStarts of [[0, 0, 16, 16, 32], [0, 16, 32], [0, 0, 16], [0, 0, 16, 16], [16]]) {
			const OffsetBuffer = sharedOffsets ? SharedArrayBuffer : ArrayBuffer;
			const columns = new Uint8Array(new OffsetBuffer(12));
			const rows = new Uint8Array(new OffsetBuffer(rowStarts.length * 4));
			const columnView = new DataView(columns.buffer);
			const rowView = new DataView(rows.buffer);
			[0, 8, 12].forEach((value, i) => columnView.setUint32(i * 4, value, true));
			rowStarts.forEach((value, i) => rowView.setUint32(i * 4, value, true));
			const expected = new Uint8Array(rowStarts.flatMap(row => [0, 8, 12].flatMap(column => [...source.subarray(row + column, row + column + 4)])));
			const sink = {};
			copies = 0;
			assert.equal(completeSparseResult(columns, rows, source, 48, expected.length, 3, rowStarts.length, sink), 0);
			assert.deepEqual(sink.value.data, expected);
			const repeatRows = expected.length > source.length && !sharedSource && !sharedOffsets && sourceOffset % 4 === 0;
			assert.equal(copies, repeatRows ? rowStarts.filter((row, i) => i > 0 && row === rowStarts[i - 1]).length : 0);
			const destination = new Uint8Array(expected.length);
			copies = 0;
			gatherInput(destination, columns, rows, source, 48);
			assert.deepEqual(destination, expected);
			assert.equal(copies, 0, 'generic gathering retains its original reads');
		}
	}
});

test('failed repeated-row copy does not publish partial direct output', (t) => {
	const columns = new Uint8Array(4);
	const rows = new Uint8Array(8);
	const source = new Uint8Array([1, 2, 3, 4]);
	t.mock.method(Uint32Array.prototype, 'copyWithin', () => { throw new Error('fixture row copy failure'); });
	const sink = {};
	assert.equal(completeSparseResult(columns, rows, source, 4, 8, 1, 2, sink), 1);
	assert.equal(sink.value, undefined);
});
