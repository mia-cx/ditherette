import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const scalar = new URL('../dist/scalar/', import.meta.url);

// These are binding checks, not public API or allocation-safety tests of the legacy methods.
test('built factories isolate real Wasm memory, cached views, and error imports', async (t) => {
	const instantiate = t.mock.method(WebAssembly, 'instantiate', () => {
		throw new Error('Factory import must not instantiate Wasm.');
	});
	const fetch = t.mock.method(globalThis, 'fetch', () => {
		throw new Error('Factory import must not fetch Wasm.');
	});
	const { createScalarBindings } = await import(new URL('ditherette_wasm.factory.js', scalar));
	const first = createScalarBindings();
	const second = createScalarBindings();
	assert.equal(instantiate.mock.callCount(), 0);
	assert.equal(fetch.mock.callCount(), 0);
	instantiate.mock.restore();
	fetch.mock.restore();
	const normal = await import(new URL('ditherette_wasm.js', scalar));
	assert.deepEqual(Object.keys(first).sort(), Object.keys(normal).sort());
	for (const name of Object.keys(first)) assert.notEqual(first[name], second[name]);

	const bytes = await readFile(new URL('ditherette_wasm_bg.wasm', scalar));
	const module = await WebAssembly.compile(bytes);
	const [firstWasm, secondWasm] = await Promise.all([
		first.default({ module_or_path: module }),
		second.default({ module_or_path: module })
	]);
	assert.notEqual(firstWasm, secondWasm);
	assert.notEqual(firstWasm.memory, secondWasm.memory);
	assert.notEqual(firstWasm.__wbindgen_externrefs, secondWasm.__wbindgen_externrefs);
	assert.equal(first.initSync({ module }), firstWasm);

	const firstPixel = new Uint8Array([64, 128, 192, 255]);
	const secondPixel = new Uint8Array([192, 128, 64, 127]);
	const resize = (bindings, pixel, filter = 'nearest', anchor = 'center') =>
		bindings.resizeRgba8(pixel, 1, 1, 2, 1, filter, anchor, 'fixed', false);
	const firstResult = resize(first, firstPixel);
	const secondResult = resize(second, secondPixel);
	assert.deepEqual(firstResult, new Uint8Array([...firstPixel, ...firstPixel]));
	assert.deepEqual(secondResult, new Uint8Array([...secondPixel, ...secondPixel]));
	const colors = first.convertColorSpace(firstPixel, 1, 1, 'rgba8', 'oklab-f32', false);
	const secondBuffer = secondWasm.memory.buffer;
	const firstBuffer = firstWasm.memory.buffer;
	firstWasm.memory.grow(1);
	assert.equal(firstBuffer.byteLength, 0);
	assert.equal(secondWasm.memory.buffer, secondBuffer);
	assert.deepEqual(resize(first, firstPixel), firstResult);
	assert.deepEqual(resize(second, secondPixel), secondResult);
	assert.deepEqual(first.convertColorSpace(firstPixel, 1, 1, 'rgba8', 'oklab-f32', false), colors);
	assert.throws(
		() => resize(first, firstPixel, 'invalid'),
		(error) => error === 'unsupported resize filter'
	);
	assert.throws(
		() => resize(second, secondPixel, 'nearest', 'invalid'),
		(error) => error === 'unsupported resize anchor'
	);
	assert.match(first.hello('first'), /first/);
	assert.match(second.hello('second'), /second/);
	assert.deepEqual(firstResult, new Uint8Array([...firstPixel, ...firstPixel]));
});

test('built factory retains byte-view, Response, and sibling default initialization', async (t) => {
	const { createScalarBindings } = await import(new URL('ditherette_wasm.factory.js', scalar));
	const bytes = await readFile(new URL('ditherette_wasm_bg.wasm', scalar));
	const padded = new Uint8Array(bytes.length + 8);
	padded.set(bytes, 4);
	const sync = createScalarBindings();
	sync.initSync({ module: padded.subarray(4, 4 + bytes.length) });
	assert.match(sync.hello('offset view'), /offset view/);
	const response = () => new Response(bytes, { headers: { 'Content-Type': 'application/wasm' } });
	const custom = createScalarBindings();
	await custom.default({ module_or_path: response() });
	assert.match(custom.hello('response'), /response/);
	const fetched = [];
	t.mock.method(globalThis, 'fetch', async (url) => {
		fetched.push(url.href);
		return response();
	});
	const defaults = createScalarBindings();
	await defaults.default();
	assert.deepEqual(fetched, [new URL('ditherette_wasm_bg.wasm', scalar).href]);
	assert.match(defaults.hello('default URL'), /default URL/);
});
