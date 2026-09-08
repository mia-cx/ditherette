import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createScalarBindings } from '../dist/scalar/ditherette_wasm.factory.js';

const module = await WebAssembly.compile(await readFile(new URL('../dist/scalar/ditherette_wasm_bg.wasm', import.meta.url)));
async function fresh(limit = 1_000_000) {
	const bindings = createScalarBindings();
	const raw = await bindings.default({ module_or_path: module });
	assert.equal(bindings.privateInitialize(limit), 0);
	return { bindings, raw };
}
const input = new Uint8Array([255, 0, 0, 128, 99, 71, 53, 0]);
const palette = [0xff0000, 0, 0x1000000];
function invoke(bindings, sink = {}, colors = palette) {
	const status = bindings.privateQuantize(input, 2, 1, colors, 0, 0, 127.9999999, 0, sink);
	if (status !== 0) assert.equal(sink.value, undefined);
	return { status, output: sink.value };
}

test('private quantize borrows its ABI and returns complete authoritative indexed metadata', async () => {
	const glue = await readFile(new URL('../dist/scalar/ditherette_wasm.js', import.meta.url), 'utf8');
	const body = glue.match(/export function privateQuantize\([^]*?\n}/)?.[0];
	assert.ok(body);
	assert.doesNotMatch(body, /__wbindgen_malloc|passArray|\.slice\(|addToExternrefTable|new Uint8Array/);
	const { bindings } = await fresh();
	assert.deepEqual(invoke(bindings), { status: 0, output: { width: 2, height: 1,
		indices: new Uint8Array([0, 2]), palette: { rgba: new Uint8Array([255,0,0,255,0,0,0,255,0,0,0,0]), transparentIndex: 2 }, warnings: [] } });
	const truncated = invoke(bindings, {}, Array(257).fill(0x1000000));
	assert.deepEqual(truncated.output.warnings, [
		{ code: 'palette-truncated', message: 'Palette was truncated to 256 entries for indexed PNG export.' },
		{ code: 'transparent-only', message: 'Only Transparent is enabled; every output pixel is transparent.' }
	]);
	assert.equal(truncated.output.palette.rgba.length, 1024);
	const length = Object.getOwnPropertyDescriptor(Uint8Array.prototype, 'length');
	let protectedOutput;
	Object.defineProperty(Uint8Array.prototype, 'length', { configurable: true, get: () => 99 });
	try { protectedOutput = invoke(bindings); }
	finally {
		if (length) Object.defineProperty(Uint8Array.prototype, 'length', length);
		else delete Uint8Array.prototype.length;
	}
	assert.equal(protectedOutput.output.indices.length, 2);
	assert.equal(protectedOutput.output.palette.rgba.length, 12);
	bindings.privateDispose();
});

test('repeated caught input, output, palette and sink failures retain no externrefs and recover', async () => {
	const { bindings, raw } = await fresh();
	const table = Object.values(raw).find(value => value instanceof WebAssembly.Table);
	const live = () => Array.from({ length: table.length }, (_, index) => table.get(index)).filter(value => value !== null).length;
	invoke(bindings);
	const baseline = { table: table.length, live: live(), bytes: raw.memory.buffer.byteLength };
	const badPalette = new Proxy(palette, { get(target, key) { if (key === '0') throw new Error('palette read'); return Reflect.get(target, key); } });
	for (let cycle = 0; cycle < 512; cycle++) {
		assert.equal(invoke(bindings).status, 0);
		for (const failAt of [1, 2, 3]) {
			const set = Uint8Array.prototype.set;
			let copies = 0;
			Uint8Array.prototype.set = function (...args) {
				if (++copies === failAt) throw new RangeError('copy failure');
				return Reflect.apply(set, this, args);
			};
			try { assert.equal(invoke(bindings).status, 9); }
			finally { Uint8Array.prototype.set = set; }
			assert.equal(bindings.privateErrorPath(), failAt === 1 ? 4 : 8);
		}
		assert.equal(invoke(bindings, Object.freeze({})).status, 9);
		assert.equal(invoke(bindings, {}, badPalette).status, 3);
	}
	assert.equal(table.length, baseline.table);
	assert.equal(live(), baseline.live);
	assert.equal(raw.memory.buffer.byteLength, baseline.bytes);
	assert.equal(invoke(bindings).status, 0);
	bindings.privateDispose();
	assert.equal(invoke(bindings).status, 10);
});
