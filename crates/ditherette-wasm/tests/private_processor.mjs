import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const distribution = new URL('../dist/scalar/', import.meta.url);
const glueUrl = new URL('ditherette_wasm.js', distribution);
const compiled = await WebAssembly.compile(await readFile(new URL('ditherette_wasm_bg.wasm', distribution)));
let instanceId = 0;
// 2×1→3×2 owns 32 pixel bytes, three Wasm usize x offsets, and two u32 y coordinates.
let resizeCapacity = 32 + 3 * 4 + 2 * 4;

// Only this low-level fixture isolates generated singleton glue with fresh import URLs.
// The shipped wrapper uses the separately tested crate-owned binding factory.
async function fresh(limit) {
	const bindings = await import(`${glueUrl.href}?fixture=${instanceId++}`);
	const raw = await bindings.default({ module_or_path: compiled });
	const overhead = bindings.privateMemoryOverhead();
	if (limit !== null) assert.equal(bindings.privateInitialize(limit ?? overhead + resizeCapacity), 0);
	return { bindings, raw, overhead };
}

const source = () => new Uint8Array([10, 20, 30, 0, 40, 50, 60, 255]);
function invoke(bindings, input, ...shape) {
	const sink = { value: undefined };
	const status = bindings.privateResize(input, ...shape.slice(0, 4), 0, shape[4], 0, sink);
	if (status !== 0) {
		assert.equal(sink.value, undefined, 'failed calls never publish a result');
		return status;
	}
	assert.ok(sink.value);
	return sink.value;
}
const resize = (bindings, input = source()) => invoke(bindings, input, 2, 1, 3, 2, 4);

// Determine the compiled preparation record using a nonidentity resize.
// The coordinate-map and source/output capacity formula above stays independent.
const { overhead: fixedOverhead } = await fresh(null);
let low = fixedOverhead;
let high = fixedOverhead + 1024;
while (low < high) {
	const limit = Math.floor((low + high) / 2);
	const { bindings } = await fresh(limit);
	const result = resize(bindings);
	if (typeof result === 'number') { assert.equal(result, 8); low = limit + 1; }
	else high = limit;
	bindings.privateDispose();
}
const resizeRecordBytes = low - fixedOverhead - resizeCapacity;
resizeCapacity += resizeRecordBytes;

function withCopyFailure(raw, phase, run) {
	const original = Uint8Array.prototype.set;
	Uint8Array.prototype.set = function (...args) {
		const intoWasm = this.buffer === raw.memory.buffer;
		if (intoWasm === (phase === 'input')) throw new RangeError(`fixture ${phase} copy failure`);
		return Reflect.apply(original, this, args);
	};
	try { return run(); }
	finally { Uint8Array.prototype.set = original; }
}

test('generated private input ABI borrows externref and catches both borrowed-slice helpers', async () => {
	const glue = await readFile(glueUrl, 'utf8');
	const body = glue.match(/export function privateResize\([^]*?\n}/)?.[0];
	assert.ok(body, 'privateResize export');
	assert.doesNotMatch(body, /__wbindgen_malloc|passArray|\.slice\(|addToExternrefTable|new Uint8Array/);
	assert.match(body, /wasm\.privateResize\(input,/);
	for (const name of ['snapshotInput', 'gatherInput', 'completeResult', 'inputLength']) {
		assert.match(glue, new RegExp(`handleError\\(function[^]*?\\b${name}\\(`), `${name} uses catch glue`);
	}
});

test('sparse nearest gathers exact Rust-selected pixels from aligned and unaligned views', async () => {
	const { bindings, raw } = await fresh(1 << 20);
	for (const [width, height, outWidth, outHeight] of [[40, 32, 10, 8], [43, 37, 7, 5], [2, 128, 4, 2]]) {
		for (const offset of [0, 1, 4]) {
			const backing = Uint8Array.from({ length: width * height * 4 + 8 }, (_, i) => (i * 73 + Math.floor(i / 251)) & 255);
			const input = backing.subarray(offset, offset + width * height * 4);
			const originalBytes = backing.slice();
			Object.defineProperty(input, 'length', { value: 123 });
			for (let anchor = 0; anchor < 9; anchor++) {
				const axis = (coordinate, source, output, alignment) => alignment === 0
					? Math.floor(coordinate * source / output)
					: alignment === 1 ? Math.floor((coordinate + 0.5) * source / output)
						: Math.floor(((coordinate + 1) * source - 1) / output);
				const expected = [];
				for (let y = 0; y < outHeight; y++) for (let x = 0; x < outWidth; x++) {
					const start = (axis(y, height, outHeight, Math.floor(anchor / 3)) * width + axis(x, width, outWidth, anchor % 3)) * 4;
					expected.push(...input.subarray(start, start + 4));
				}
				const result = withCopyFailure(raw, 'input', () => invoke(bindings, input, width, height, outWidth, outHeight, anchor));
				assert.deepEqual([...result.data], expected);
				assert.notEqual(result.data.buffer, raw.memory.buffer);
			}
			assert.deepEqual(backing, originalBytes);
		}
	}
	bindings.privateDispose();
});

test('sparse nearest observes mutations across full-source transitions and durable output survives disposal', async () => {
	const { bindings, raw } = await fresh(1 << 20);
	const input = Uint8Array.from({ length: 64 * 64 * 4 }, (_, i) => i & 255);
	const full = () => invoke(bindings, input, 64, 64, 32, 32, 0);
	const sparse = () => invoke(bindings, input, 64, 64, 4, 4, 0);
	full();
	full();
	const durable = sparse();
	input[0] = 99;
	assert.equal(sparse().data[0], 99);
	assert.equal(full().data[0], 99);
	input[0] = 77;
	assert.equal(sparse().data[0], 77);
	assert.equal(withCopyFailure(raw, 'result', sparse), 9);
	assert.equal(bindings.privateErrorPath(), 8);
	assert.equal(sparse().data[0], 77);
	raw.memory.grow(1);
	bindings.privateDispose();
	assert.equal(durable.data[0], 0);
	assert.equal(sparse(), 10);
});

test('sparse gather catches detached storage, thrown imports, and callback failures without leaking handles', async () => {
	const { bindings, raw } = await fresh(1 << 20);
	let input = new Uint8Array(64 * 64 * 4).fill(71);
	const run = sink => bindings.privateResize(input, 64, 64, 4, 4, 0, 4, 0, sink);
	const table = Object.values(raw).find(value => value instanceof WebAssembly.Table);
	const live = () => Array.from({ length: table.length }, (_, index) => table.get(index)).filter(value => value !== null).length;
	assert.equal(run({}), 0);
	const capacity = table.length;
	const initialLive = live();
	const pages = raw.memory.buffer.byteLength;
	const original = DataView.prototype.getUint32;
	for (let i = 0; i < 32; i++) {
		const failed = {};
		DataView.prototype.getUint32 = function () { throw new RangeError('fixture gather failure'); };
		try { assert.equal(run(failed), 9); }
		finally { DataView.prototype.getUint32 = original; }
		assert.equal(bindings.privateErrorPath(), 4);
		assert.equal(failed.value, undefined);
		const callback = { onProgress(event) {
			assert.equal(bindings.privateDispose(), 11);
			assert.equal(run({}), 11);
			if (event.stage === 'complete') throw new Error('fixture completion failure');
		} };
		assert.equal(run(callback), 12);
		assert.equal(callback.value, undefined);
		const recovered = {};
		assert.equal(run(recovered), 0);
		assert.ok(recovered.value.data.every(value => value === 71));
	}
	assert.equal(table.length, capacity);
	assert.equal(live(), initialLive);
	assert.equal(raw.memory.buffer.byteLength, pages);
	const detachedDuringPrepare = { onProgress(event) {
		if (event.stage === 'prepare') structuredClone(input.buffer, { transfer: [input.buffer] });
	} };
	assert.equal(run(detachedDuringPrepare), 9);
	assert.equal(bindings.privateErrorPath(), 4);
	assert.equal(detachedDuringPrepare.value, undefined);
	assert.equal(run({}), 2);
	input = new Uint8Array(64 * 64 * 4).fill(17);
	assert.equal(run({}), 0);
	bindings.privateDispose();
});

test('exact capacity, one-under preflight, and tiny initialization have stable errors', async () => {
	const { bindings, overhead } = await fresh(null);
	assert.equal(bindings.privateInitialize(1), 8);
	assert.equal(bindings.privateErrorPath(), 1);
	for (const invalid of [0, -1, 1.5, NaN, Infinity, 2147483649]) {
		assert.equal(bindings.privateInitialize(invalid), 4);
		assert.equal(bindings.privateErrorPath(), 1);
	}
	assert.equal(bindings.privateInitialize(overhead + resizeCapacity), 0);
	assert.equal(resize(bindings).data.length, 24);
	const short = await fresh(overhead + resizeCapacity - 1);
	let copies = 0;
	const original = Uint8Array.prototype.set;
	Uint8Array.prototype.set = function (...args) {
		copies++;
		return Reflect.apply(original, this, args);
	};
	try {
		assert.equal(resize(short.bindings), 8);
		assert.equal(short.bindings.privateErrorPath(), 1);
		assert.equal(copies, 1); // Hash the owned snapshot before remaining execution preflight.
	} finally { Uint8Array.prototype.set = original; }
	assert.equal(invoke(short.bindings, source(), 2, 1, 1, 1, 4).data.length, 4);
});

test('offset and spoofed-length inputs preserve bytes; results survive reuse, growth, and disposal', async () => {
	const { bindings, raw } = await fresh();
	const backing = new Uint8Array([99, 98, ...source(), 97, 96]);
	const offset = backing.subarray(2, 10);
	Object.defineProperty(offset, 'length', { value: 123 });
	const first = resize(bindings, offset);
	assert.deepEqual([...first.data], [10,20,30,0,40,50,60,255,40,50,60,255,10,20,30,0,40,50,60,255,40,50,60,255]);
	assert.deepEqual([...backing], [99,98,10,20,30,0,40,50,60,255,97,96]);
	assert.notEqual(first.data.buffer, raw.memory.buffer);
	resize(bindings, new Uint8Array(8).fill(77));
	raw.memory.grow(1);
	assert.equal(first.data[0], 10);
	assert.equal(bindings.privateDispose(), 0);
	assert.equal(bindings.privateDispose(), 0);
	assert.equal(resize(bindings), 10);
	assert.equal(bindings.privateErrorPath(), 0);
	assert.equal(first.data[4], 40);
	const other = await fresh();
	assert.equal(resize(other.bindings).data[0], 10);
});

test('invalid dimensions, detached storage, and non-byte arrays fail without poisoning the instance', async () => {
	const { bindings } = await fresh();
	for (const width of [0, -1, 1.5, NaN, Infinity, 4294967297]) {
		assert.equal(invoke(bindings, source(), width, 1, 1, 1, 4), 2);
		assert.equal(bindings.privateErrorPath(), 2);
	}
	for (const anchor of [-1, 0.5, 9, NaN]) {
		assert.equal(invoke(bindings, source(), 2, 1, 1, 1, anchor), 4);
		assert.equal(bindings.privateErrorPath(), 9);
	}
	const detached = source();
	structuredClone(detached.buffer, { transfer: [detached.buffer] });
	for (const input of [detached, new Uint16Array(4), source().subarray(1)]) {
		assert.equal(resize(bindings, input), 2);
		assert.equal(bindings.privateErrorPath(), 4);
	}
	assert.equal(resize(bindings).data.length, 24);
});

test('caught copy failures and recursive calls recover without mutable glue borrows', async () => {
	const { bindings, raw } = await fresh();
	for (const [phase, path] of [['input', 4], ['result', 8]]) {
		assert.equal(withCopyFailure(raw, phase, () => resize(bindings)), 9);
		assert.equal(bindings.privateErrorPath(), path);
		assert.equal(resize(bindings).data.length, 24);
	}
	const frozenSink = Object.freeze({});
	assert.equal(bindings.privateResize(source(), 2, 1, 3, 2, 0, 4, 0, frozenSink), 9);
	assert.equal(bindings.privateErrorPath(), 8);
	assert.equal(frozenSink.value, undefined);
	assert.equal(resize(bindings).data.length, 24);
	const original = Uint8Array.prototype.set;
	let recursed = false;
	Uint8Array.prototype.set = function (...args) {
		if (!recursed && this.buffer === raw.memory.buffer) {
			recursed = true;
			assert.equal(bindings.privateDispose(), 11);
			assert.equal(bindings.privateInitialize(1610612736), 11);
			assert.equal(resize(bindings), 11);
			assert.equal(bindings.privateErrorPath(), 0);
		}
		return Reflect.apply(original, this, args);
	};
	try { assert.equal(resize(bindings, new Uint8Array(8).fill(17)).data.length, 24); }
	finally { Uint8Array.prototype.set = original; }
	assert.equal(recursed, true);
	assert.equal(resize(bindings).data.length, 24);
});

test('repeated success and caught failures keep externref capacity and live handles bounded', async () => {
	const { bindings, raw } = await fresh();
	const table = Object.values(raw).find(value => value instanceof WebAssembly.Table);
	assert.ok(table);
	const live = () => Array.from({ length: table.length }, (_, index) => table.get(index))
		.filter(value => value !== null).length;
	const capacity = table.length;
	const initialLive = live();
	resize(bindings);
	const pages = raw.memory.buffer.byteLength;
	for (let call = 0; call < 512; call++) {
		assert.equal(resize(bindings).data.length, 24);
		assert.equal(withCopyFailure(raw, 'input', () => resize(bindings, new Uint8Array(8).fill(17))), 9);
		assert.equal(withCopyFailure(raw, 'result', () => resize(bindings)), 9);
	}
	assert.equal(table.length, capacity);
	assert.equal(live(), initialLive);
	assert.equal(raw.memory.buffer.byteLength, pages);
});

test('caught void progress rejects reentry and completion failure without retaining handles', async () => {
	const { bindings, raw } = await fresh(4 << 20);
	const input = source();
	let failStage;
	const sink = { value: undefined, onProgress(event) {
		assert.equal(bindings.privateDispose(), 11);
		assert.equal(bindings.privateResize(input, 2, 1, 3, 2, 0, 4, 0, {}), 11);
		if (event.stage === 'complete') assert.equal(sink.value.data.length, 24);
		if (event.stage === failStage) throw new Error('fixture progress failure');
	} };
	const call = () => {
		sink.value = undefined;
		return bindings.privateResize(input, 2, 1, 3, 2, 0, 4, 0, sink);
	};
	assert.equal(call(), 0);
	const table = Object.values(raw).find(value => value instanceof WebAssembly.Table);
	const live = () => Array.from({ length: table.length }, (_, index) => table.get(index))
		.filter(value => value !== null).length;
	const capacity = table.length;
	const initialLive = live();
	const pages = raw.memory.buffer.byteLength;
	for (let iteration = 0; iteration < 512; iteration++) {
		for (const stage of ['prepare', 'complete']) {
			failStage = stage;
			assert.equal(call(), 12);
			assert.equal(bindings.privateErrorPath(), 38);
			assert.equal(sink.value, undefined);
		}
		failStage = undefined;
		assert.equal(call(), 0);
	}
	assert.equal(table.length, capacity);
	assert.equal(live(), initialLive);
	assert.equal(raw.memory.buffer.byteLength, pages);
	const glue = await readFile(glueUrl, 'utf8');
	for (const name of ['progressEnabled', 'progressClock', 'reportProgress'])
		assert.match(glue, new RegExp(`handleError\\(function[^]*?\\b${name}\\(`));
	bindings.privateDispose();
});

test('convolution ABI preserves landed output for every policy and anchor and rejects invalid discriminators', async () => {
	const { bindings } = await fresh(10_000_000);
	const input = new Uint8Array(Array.from({ length: 7 * 5 * 4 }, (_, i) => (i * 73) % 256));
	const anchors = ['top-left', 'top', 'top-right', 'left', 'center', 'right', 'bottom-left', 'bottom', 'bottom-right'];
	for (const [mode, algorithm] of ['bicubic', 'lanczos2', 'lanczos3'].entries()) {
		for (const [support, name] of ['fixed', 'scale-aware'].entries()) {
			for (const [anchor, label] of anchors.entries()) {
				const sink = {};
				assert.equal(bindings.privateResize(input, 7, 5, 3, 2, mode + 3, anchor, support, sink), 0);
				const landed = bindings.resizeRgba8(input, 7, 5, 3, 2, algorithm, label, name, false);
				assert.deepEqual(sink.value.data, landed, `${algorithm} ${name} ${label}`);
			}
		}
	}
	for (const algorithm of [0, 1, 2, 3, 4, 5, 6]) {
		for (const support of [-1, 0.5, 2, NaN, Infinity, ...(algorithm < 3 || algorithm === 6 ? [1] : [])]) {
			const sink = {};
			assert.equal(bindings.privateResize(source(), 2, 1, 1, 1, algorithm, algorithm === 1 ? 0 : 4, support, sink), 4);
			assert.equal(bindings.privateErrorPath(), 12);
			assert.equal(sink.value, undefined);
		}
	}
	const sink = {};
	assert.equal(bindings.privateResize(source(), 2, 1, 1, 1, 1, 4, 0, sink), 4);
	assert.equal(bindings.privateErrorPath(), 9);
	assert.equal(sink.value, undefined);
	for (const algorithm of [-1, 0.5, 7, NaN, Infinity]) {
		assert.equal(bindings.privateResize(source(), 2, 1, 1, 1, algorithm, 4, 0, {}), 4);
	}
	assert.equal(resize(bindings).data.length, 24);
	bindings.privateDispose();
});

test('trilinear exact budget, mip rounding, caught failures, and recovery use the borrowed ABI', async () => {
	// 4x1→1x1 owns 20 input/output bytes, three 20-byte Wasm MipLevel headers,
	// 16+8+4 mip bytes, four f64 accumulators, and the owned preparation record.
	const capacity = resizeRecordBytes + 20 + 3 * 20 + 16 + 8 + 4 + 32;
	const initial = await fresh(null);
	assert.equal(initial.bindings.privateInitialize(initial.overhead + capacity), 0);
	const { bindings, raw } = initial;
	const pixels = new Uint8Array(16);
	pixels.fill(1, 12);
	const call = (target = bindings) => {
		const sink = {};
		const status = target.privateResize(pixels, 4, 1, 1, 1, 6, 4, 0, sink);
		if (status !== 0) assert.equal(sink.value, undefined);
		else assert.deepEqual([...sink.value.data], [1, 1, 1, 1]);
		return status;
	};
	assert.equal(call(), 0);
	const short = await fresh(initial.overhead + capacity - 1);
	assert.equal(call(short.bindings), 8);
	const table = Object.values(raw).find(value => value instanceof WebAssembly.Table);
	const length = table.length;
	const pages = raw.memory.buffer.byteLength;
	for (let i = 0; i < 32; i++) {
		for (const phase of ['input', 'result']) {
			// Equal snapshots skip input copying; change a byte to reach the caught copy.
			if (phase === 'input') pixels[0] = 1;
			assert.equal(withCopyFailure(raw, phase, call), 9);
			pixels[0] = 0;
			assert.equal(call(), 0);
		}
	}
	assert.equal(table.length, length);
	assert.equal(raw.memory.buffer.byteLength, pages);
	bindings.privateDispose();
	short.bindings.privateDispose();
});
