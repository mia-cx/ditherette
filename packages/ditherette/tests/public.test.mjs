import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette, DitheretteError } from '../dist/index.js';
import { createScalarBindings } from '../dist/wasm/scalar/ditherette_wasm.factory.js';
import { initializeProcessor } from '../dist/scalar.js';

// Node is a development fixture host, not a supported public runtime.
const bytes = await readFile(
	new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url)
);
const module = await WebAssembly.compile(bytes);
const privateBindings = createScalarBindings();
await privateBindings.default({ module_or_path: module });
const overhead = privateBindings.privateMemoryOverhead();
const request = (data = new Uint8Array([17, 31, 47, 127])) => ({
	version: 1,
	source: { width: 1, height: 1, data },
	output: { width: 2, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } }
});
const diagnostic = (code, path) => (error) =>
	error instanceof DitheretteError && error.code === code && error.path === path;

// A 1x1 -> 2x1 nearest resize owns its preparation record, 12 pixel bytes and 12 map bytes.
// Identity has no preparation now. Probe a real resize; other filter formulas stay independent.
const resizeOverhead = await (async () => {
	let low = overhead;
	let high = overhead + 1024;
	while (low < high) {
		const limit = Math.floor((low + high) / 2);
		const processor = await createDitherette({ wasm: module, memoryLimitBytes: limit });
		try {
			const value = request();
			processor.resize(value);
			high = limit;
		} catch (error) {
			assert.ok(diagnostic('memory-limit', 'memoryLimitBytes')(error));
			low = limit + 1;
		} finally {
			processor.dispose();
		}
	}
	return low - 24;
})();

test('identity resize returns the input object and aliases its bytes', async () => {
	const value = request();
	value.output.width = 1;
	const processor = await createDitherette({ wasm: module, memoryLimitBytes: overhead + 4 });
	try {
		const result = processor.resize(value);
		assert.equal(result, value.source);
		result.data.fill(0);
		assert.deepEqual([...value.source.data], [0, 0, 0, 0]);
		assert.deepEqual(processor.resize(value).data, value.source.data);
		value.source.data[0] = 99;
		assert.equal(processor.resize(value).data[0], 99);
	} finally {
		processor.dispose();
	}
});

test('identity bypass validates requests and preserves lifecycle without entering Wasm', () => {
	const processor = initializeProcessor({
		privateInitialize: () => 0,
		privateDispose: () => 0,
		privateResize: () => assert.fail('identity must not enter Wasm')
	}, 1);
	const value = request(new Uint8Array([9, 17, 31, 47, 127, 9]).subarray(1, 5));
	value.output.width = 1;
	for (const algorithm of ['nearest', 'area', 'bilinear', 'bicubic', 'lanczos2', 'lanczos3', 'trilinear']) {
		value.output.resize = algorithm === 'area' ? { algorithm } : {
			algorithm, anchor: 'center',
			...(['bicubic', 'lanczos2', 'lanczos3'].includes(algorithm) ? { support: 'fixed' } : {})
		};
		assert.equal(processor.resize(value), value.source);
	}
	value.onProgress = event => {
		assert.deepEqual(event, { stage: 'complete' });
		assert.throws(() => processor.resize(value), diagnostic('reentrant-call', 'instance'));
		assert.throws(() => processor.dispose(), diagnostic('reentrant-call', 'instance'));
	};
	assert.equal(processor.resize(value), value.source);
	value.onProgress = () => { throw new Error('callback failed'); };
	assert.throws(() => processor.resize(value), diagnostic('callback', 'onProgress'));
	delete value.onProgress;
	assert.equal(processor.resize(value), value.source);
	assert.throws(() => processor.resize({ ...value, version: 0 }), diagnostic('invalid-request', 'version'));
	processor.dispose();
	assert.throws(() => processor.resize(value), diagnostic('disposed', 'instance'));
});

test('public trilinear preserves intermediate rounding and recovers from budget and copy failures', async () => {
	// Wasm mip headers, chain bytes, f64 channels, and imported source/output capacities.
	const capacity = 3 * 20 + 16 + 8 + 4 + 32 + 16 + 4;
	const processor = await createDitherette({
		wasm: module,
		memoryLimitBytes: resizeOverhead + capacity
	});
	const short = await createDitherette({
		wasm: module,
		memoryLimitBytes: resizeOverhead + capacity - 1
	});
	const backing = new Uint8Array([99, ...new Uint8Array(12), 1, 1, 1, 1, 98]);
	const value = {
		version: 1,
		source: { width: 4, height: 1, data: backing.subarray(1, 17) },
		output: { width: 1, height: 1, resize: { algorithm: 'trilinear', anchor: 'center' } }
	};
	const original = [...backing];
	const result = processor.resize(value);
	assert.deepEqual([...result.data], [1, 1, 1, 1]);
	assert.throws(() => short.resize(value), diagnostic('memory-limit', 'memoryLimitBytes'));
	const small = { ...value, source: { width: 1, height: 1, data: new Uint8Array([7, 8, 9, 0]) } };
	assert.deepEqual([...short.resize(small).data], [7, 8, 9, 0]);
	for (const phase of [1, 2]) {
		const changed = structuredClone(value);
		changed.source.data[0] ^= 1;
		processor.resize(changed);
		const originalSet = Uint8Array.prototype.set;
		let calls = 0;
		Uint8Array.prototype.set = function (...args) {
			if (++calls === phase) throw new RangeError('trilinear copy failure');
			return Reflect.apply(originalSet, this, args);
		};
		try {
			assert.throws(
				() => processor.resize(value),
				diagnostic('wasm-memory-unavailable', phase === 1 ? 'source.data' : 'output')
			);
		} finally {
			Uint8Array.prototype.set = originalSet;
		}
		assert.deepEqual([...processor.resize(value).data], [1, 1, 1, 1]);
	}
	processor.dispose();
	short.dispose();
	assert.deepEqual([...result.data], [1, 1, 1, 1]);
	assert.deepEqual([...backing], original);
});

test('public calls preserve inputs and durable outputs across growth, later calls, and disposal', async (t) => {
	const memories = [];
	const instantiate = WebAssembly.instantiate;
	t.mock.method(WebAssembly, 'instantiate', async (...args) => {
		const result = await instantiate(...args);
		memories.push(
			(result instanceof WebAssembly.Instance ? result : result.instance).exports.memory
		);
		return result;
	});
	const processor = await createDitherette({ wasm: module });
	const backing = new Uint8Array([9, 17, 31, 47, 127, 9]);
	const input = backing.subarray(1, 5);
	const output = processor.resize(request(input));
	assert.deepEqual(output, {
		width: 2,
		height: 1,
		data: new Uint8Array([17, 31, 47, 127, 17, 31, 47, 127])
	});
	assert.deepEqual(backing, new Uint8Array([9, 17, 31, 47, 127, 9]));
	assert.notEqual(output.data.buffer, input.buffer);
	assert.notEqual(output.data.buffer, memories[0].buffer);
	memories[0].grow(1);
	input[0] = 211;
	const later = processor.resize(request(input));
	assert.equal(later.data[0], 211);
	assert.equal(output.data[0], 17);
	later.data[0] = 2;
	assert.equal(processor.resize(request(input)).data[0], 211);
	processor.dispose();
	processor.dispose();
	assert.equal(output.data[0], 17);
	assert.throws(() => processor.resize(request()), diagnostic('disposed', 'instance'));
});

test('public area and bilinear preserve hidden RGB, alpha, exact budgets, and recovery', async () => {
	for (const [algorithm, capacity] of [
		['area', 12],
		['bilinear', 116]
	]) {
		// Bilinear adds two 12-byte Vec headers and five 16-byte taps in Wasm32.
		const resize = algorithm === 'area' ? { algorithm } : { algorithm, anchor: 'center' };
		const backing = new Uint8Array([9, 200, 0, 100, 0, 0, 100, 200, 255, 9]);
		const value = {
			version: 1,
			source: { width: 2, height: 1, data: backing.subarray(1, 9) },
			output: { width: 1, height: 1, resize }
		};
		const processor = await createDitherette({
			wasm: module,
			memoryLimitBytes: resizeOverhead + capacity
		});
		const output = processor.resize(value);
		assert.deepEqual([...output.data], [100, 50, 150, 128]);
		assert.deepEqual([...backing], [9, 200, 0, 100, 0, 0, 100, 200, 255, 9]);
		const changed = structuredClone(value);
		changed.source.data[0] ^= 1;
		processor.resize(changed);
		const originalSet = Uint8Array.prototype.set;
		let calls = 0;
		try {
			Uint8Array.prototype.set = function (...args) {
				if (++calls === 2) throw new RangeError('durable output failure');
				return Reflect.apply(originalSet, this, args);
			};
			assert.throws(() => processor.resize(value), diagnostic('wasm-memory-unavailable', 'output'));
		} finally {
			Uint8Array.prototype.set = originalSet;
		}
		assert.deepEqual([...processor.resize(value).data], [100, 50, 150, 128]);
		processor.dispose();
		assert.deepEqual([...output.data], [100, 50, 150, 128]);
		const short = await createDitherette({
			wasm: module,
			memoryLimitBytes: resizeOverhead + capacity - 1
		});
		assert.throws(() => short.resize(value), diagnostic('memory-limit', 'memoryLimitBytes'));
		const smaller = request();
		smaller.output.width = 1;
		assert.deepEqual([...short.resize(smaller).data], [17, 31, 47, 127]);
		short.dispose();
	}
});

test('one exact capacity budget succeeds and one byte less rejects without poisoning the instance', async () => {
	// 12 pixel bytes plus two Wasm usize x offsets and one u32 y coordinate.
	const capacity = 12 + 2 * 4 + 4;
	const exact = await createDitherette({
		wasm: module,
		memoryLimitBytes: resizeOverhead + capacity
	});
	assert.equal(exact.resize(request()).data.length, 8);
	exact.dispose();
	const short = await createDitherette({
		wasm: module,
		memoryLimitBytes: resizeOverhead + capacity - 1
	});
	assert.throws(() => short.resize(request()), diagnostic('memory-limit', 'memoryLimitBytes'));
	const smaller = request();
	smaller.output.width = 1;
	assert.equal(short.resize(smaller).data.length, 4);
	short.dispose();
	await assert.rejects(
		createDitherette({ wasm: module, memoryLimitBytes: overhead - 1 }),
		diagnostic('memory-limit', 'memoryLimitBytes')
	);
});

test('public convolution preserves alpha and output ownership with bounded preparation', async () => {
	for (const algorithm of ['bicubic', 'lanczos2', 'lanczos3']) {
		for (const support of ['fixed', 'scale-aware']) {
			const radius = algorithm === 'lanczos3' ? 3 : 2;
			// Two Wasm32 Vec headers, 16-byte aligned taps, and 12 input/output bytes.
			const xTaps = 2 * radius * (support === 'fixed' ? 1 : 2) + 2;
			const yTaps = 2 * radius + 2;
			const capacity = 12 + 2 * 12 + (xTaps + yTaps) * 16;
			const backing = new Uint8Array([9, 200, 0, 100, 0, 0, 100, 200, 255, 9]);
			const value = {
				version: 1,
				source: { width: 2, height: 1, data: backing.subarray(1, 9) },
				output: { width: 1, height: 1, resize: { algorithm, anchor: 'center', support } }
			};
			const processor = await createDitherette({
				wasm: module,
				memoryLimitBytes: resizeOverhead + capacity
			});
			const output = processor.resize(value);
			// The landed Wasm Lanczos3 accumulation rounds this half-byte downward for scale-aware support.
			const expected = [
				100,
				50,
				150,
				algorithm === 'lanczos3' && support === 'scale-aware' ? 127 : 128
			];
			assert.deepEqual([...output.data], expected, `${algorithm} ${support}`);
			assert.deepEqual([...backing], [9, 200, 0, 100, 0, 0, 100, 200, 255, 9]);
			const originalSet = Uint8Array.prototype.set;
			for (const phase of [1, 2]) {
				const changed = structuredClone(value);
				changed.source.data[0] ^= 1;
				processor.resize(changed);
				let calls = 0;
				try {
					Uint8Array.prototype.set = function (...args) {
						if (++calls === phase) throw new RangeError('convolution copy failure');
						return Reflect.apply(originalSet, this, args);
					};
					assert.throws(
						() => processor.resize(value),
						diagnostic('wasm-memory-unavailable', phase === 1 ? 'source.data' : 'output')
					);
				} finally {
					Uint8Array.prototype.set = originalSet;
				}
				assert.deepEqual(processor.resize(value), output);
			}
			processor.dispose();
			assert.deepEqual([...output.data], expected);
			const short = await createDitherette({
				wasm: module,
				memoryLimitBytes: resizeOverhead + capacity - 1
			});
			assert.throws(() => short.resize(value), diagnostic('memory-limit', 'memoryLimitBytes'));
			value.output.width = 2;
			assert.deepEqual([...short.resize(value).data], [...value.source.data]);
			short.dispose();
		}
	}
});

test('concurrent initialization, custom byte views, and reusable responses remain independent', async () => {
	const padded = new Uint8Array(bytes.length + 10);
	padded.set(bytes, 5);
	const response = new Response(bytes, { headers: { 'Content-Type': 'application/wasm' } });
	const [first, second, third, fourth, fifth] = await Promise.all([
		createDitherette({ wasm: padded.subarray(5, 5 + bytes.length) }),
		createDitherette({ wasm: module }),
		createDitherette({ wasm: response }),
		createDitherette({ wasm: response }),
		createDitherette({ wasm: new DataView(padded.buffer, 5, bytes.length) })
	]);
	assert.equal(response.bodyUsed, false);
	first.dispose();
	for (const processor of [second, third, fourth, fifth]) {
		assert.equal(processor.resize(request()).data[0], 17);
		processor.dispose();
	}
	const [failed, healthy] = await Promise.allSettled([
		createDitherette({ wasm: new Uint8Array([1, 2, 3]) }),
		createDitherette({ wasm: module })
	]);
	assert.equal(failed.status, 'rejected');
	assert.ok(diagnostic('initialization', 'wasm')(failed.reason));
	assert.equal(healthy.status, 'fulfilled');
	assert.equal(healthy.value.resize(request()).data[0], 17);
	healthy.value.dispose();
});

test('raw request failures and property-triggered recursion are structured before entering Wasm', async () => {
	const processor = await createDitherette({ wasm: module });
	const value = request();
	Object.defineProperty(value, 'version', {
		get() {
			assert.throws(() => processor.resize(request()), diagnostic('reentrant-call', 'instance'));
			assert.throws(() => processor.dispose(), diagnostic('reentrant-call', 'instance'));
			return 1;
		}
	});
	assert.equal(processor.resize(value).data.length, 8);
	for (const anchor of [['center'], { center: null }, null]) {
		const invalid = request();
		invalid.output.resize.anchor = anchor;
		assert.throws(
			() => processor.resize(invalid),
			diagnostic('invalid-settings', 'output.resize.anchor')
		);
	}
	const detached = request();
	structuredClone(detached.source.data.buffer, { transfer: [detached.source.data.buffer] });
	assert.throws(() => processor.resize(detached), diagnostic('invalid-image', 'source.data'));
	const callback = request();
	callback.onProgress = () => { throw new Error('fixture callback failure'); };
	assert.throws(
		() => processor.resize(callback),
		diagnostic('callback', 'onProgress')
	);
	assert.equal(processor.resize(request()).data[0], 17);
	processor.dispose();
});

test('validated callbacks are read once and reject reentry while allowing recovery', async () => {
	const processor = await createDitherette({ wasm: module });
	const value = request();
	let reads = 0;
	const events = [];
	Object.defineProperty(value, 'onProgress', { enumerable: true, get() {
		reads++;
		return (event) => {
			events.push(event);
			assert.throws(() => processor.resize(request()), diagnostic('reentrant-call', 'instance'));
			assert.throws(() => processor.dispose(), diagnostic('reentrant-call', 'instance'));
		};
	} });
	assert.equal(processor.resize(value).data.length, 8);
	assert.equal(reads, 1);
	assert.equal(events.at(-1).stage, 'complete');
	assert.deepEqual(events.at(-1), { stage: 'complete', completed: 1, total: 1 });
	assert.throws(() => processor.resize({ ...request(), onProgress(event) {
		if (event.stage === 'complete') throw new Error('completion failure');
	} }), diagnostic('callback', 'onProgress'));
	assert.equal(processor.resize(request()).data.length, 8);
	processor.dispose();
});

test('unsupported capabilities and invalid options do not silently select another public contract', async () => {
	await assert.rejects(
		createDitherette({ wasm: module, threads: 'required' }),
		diagnostic('capability', 'threads')
	);
	await assert.rejects(
		createDitherette({ wasm: module, memoryLimitBytes: 0 }),
		diagnostic('invalid-settings', 'memoryLimitBytes')
	);
	const preferred = await createDitherette({ wasm: module, threads: 'preferred' });
	assert.equal(preferred.resize(request()).data[0], 17);
	assert.equal('wasm' in preferred, false);
	assert.equal('cache' in preferred, false);
	preferred.dispose();
});

test('thread selection checks blocking-wait permission without probing disabled calls', async (t) => {
	for (const [name, value] of [['crossOriginIsolated', true], ['Worker', class {
		constructor() { assert.fail('Incapable contexts must not create workers.'); }
	}]]) {
		const previous = Object.getOwnPropertyDescriptor(globalThis, name);
		Object.defineProperty(globalThis, name, { configurable: true, value });
		t.after(() => {
			if (previous) Object.defineProperty(globalThis, name, previous);
			else delete globalThis[name];
		});
	}
	let probes = 0;
	t.mock.method(Atomics, 'wait', (view, index, expected, timeout) => {
		probes++;
		assert.ok(view.buffer instanceof SharedArrayBuffer);
		assert.deepEqual([view.length, index, expected, timeout], [1, 0, 0, 0]);
		throw new TypeError('Atomics.wait cannot be called in this context');
	});
	const disabled = await createDitherette({ wasm: module, threads: 'disabled' });
	disabled.dispose();
	assert.equal(probes, 0);
	await assert.rejects(createDitherette({ wasm: module, threads: 'required' }), diagnostic('capability', 'threads'));
	const preferred = await createDitherette({ wasm: module, threads: 'preferred' });
	assert.equal(preferred.resize(request()).data[0], 17);
	preferred.dispose();
	assert.equal(probes, 2);
});

test('unexpected initialization allocation failures are structured and do not poison later creates', async (t) => {
	const allocation = t.mock.method(WebAssembly, 'instantiate', async () => {
		throw new RangeError('injected browser allocation failure');
	});
	await assert.rejects(
		createDitherette({ wasm: module }),
		diagnostic('wasm-memory-unavailable', 'wasm')
	);
	allocation.mock.restore();
	const processor = await createDitherette({ wasm: module });
	assert.equal(processor.resize(request()).data.length, 8);
	processor.dispose();
});

test('caught input/result copy failures recover through the public boundary without externref growth', async (t) => {
	let raw;
	const instantiate = WebAssembly.instantiate;
	const capture = t.mock.method(WebAssembly, 'instantiate', async (...args) => {
		const result = await instantiate(...args);
		raw = (result instanceof WebAssembly.Instance ? result : result.instance).exports;
		return result;
	});
	const processor = await createDitherette({ wasm: module });
	capture.mock.restore();
	const capacity = raw.__wbindgen_externrefs.length;
	const previous = processor.resize(request());
	for (const [phase, path] of [
		['input', 'source.data'],
		['result', 'output']
	]) {
		const changed = request();
		changed.source.data[0] ^= 1;
		processor.resize(changed);
		const set = Uint8Array.prototype.set;
		const fault = t.mock.method(Uint8Array.prototype, 'set', function (...args) {
			const intoWasm = this.buffer === raw.memory.buffer;
			if (intoWasm === (phase === 'input')) {
				assert.throws(() => processor.resize(request()), diagnostic('reentrant-call', 'instance'));
				assert.throws(() => processor.dispose(), diagnostic('reentrant-call', 'instance'));
				throw new RangeError('injected copy failure');
			}
			return Reflect.apply(set, this, args);
		});
		for (let attempt = 0; attempt < 16; attempt++) {
			assert.throws(() => processor.resize(request()), diagnostic('wasm-memory-unavailable', path));
		}
		fault.mock.restore();
		assert.equal(processor.resize(request()).data[0], 17);
		assert.equal(raw.__wbindgen_externrefs.length, capacity);
		assert.equal(previous.data[0], 17);
	}
	processor.dispose();
});
