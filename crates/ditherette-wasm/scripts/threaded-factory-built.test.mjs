import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

test('threaded factories isolate actual shared memories and receive their own pinned pool builders', async () => {
	const directory = new URL('../dist/threads/', import.meta.url);
	const { createThreadedBindings } = await import(new URL('ditherette_wasm.factory.js', directory));
	const module = await WebAssembly.compile(await readFile(new URL('ditherette_wasm_bg.wasm', directory)));
	const pools = [];
	const first = createThreadedBindings(async (code, memory, builder) => {
		pools.push({ code, memory, count: builder.numThreads() });
		builder.free();
	});
	const second = createThreadedBindings(async (code, memory, builder) => {
		pools.push({ code, memory, count: builder.numThreads() });
		builder.free();
	});
	const [a, b] = await Promise.all([
		first.default({ module_or_path: module }), second.default({ module_or_path: module })
	]);
	assert.ok(a.memory.buffer instanceof SharedArrayBuffer);
	assert.ok(b.memory.buffer instanceof SharedArrayBuffer);
	assert.notEqual(a.memory, b.memory);
	assert.equal(first.privateInitialize(1_000_000), 0);
	assert.equal(second.privateInitialize(2_000_000), 0);
	for (const [available, expected] of [[0, 1], [1, 1], [4, 2], [32, 8]])
		assert.equal(first.privateThreadCount(available), expected);
	await first.initThreadPool(1);
	await second.initThreadPool(2);
	assert.deepEqual(pools.map(({ count }) => count), [1, 2]);
	assert.equal(pools[0].memory, a.memory);
	assert.equal(pools[1].memory, b.memory);
	assert.equal(first.privateDispose(), 0);
	assert.equal(second.privateDispose(), 0);
});
