import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette, DitheretteError } from '../dist/index.js';
import { createScalarBindings } from '../dist/wasm/scalar/ditherette_wasm.factory.js';

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

test('one exact capacity budget succeeds and one byte less rejects without poisoning the instance', async () => {
	// 12 pixel bytes plus two Wasm usize x offsets and one u32 y coordinate.
	const capacity = 12 + 2 * 4 + 4;
	const exact = await createDitherette({ wasm: module, memoryLimitBytes: overhead + capacity });
	assert.equal(exact.resize(request()).data.length, 8);
	exact.dispose();
	const short = await createDitherette({ wasm: module, memoryLimitBytes: overhead + capacity - 1 });
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
	callback.onProgress = () => assert.fail('unsupported callback must not run');
	assert.throws(
		() => processor.resize(callback),
		diagnostic('unsupported-operation', 'onProgress')
	);
	assert.equal(processor.resize(request()).data[0], 17);
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
	assert.equal('process' in preferred, false);
	assert.equal('wasm' in preferred, false);
	assert.equal('cache' in preferred, false);
	preferred.dispose();
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
