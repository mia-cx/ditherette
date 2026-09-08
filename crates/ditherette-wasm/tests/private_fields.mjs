import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createScalarBindings } from '../dist/scalar/ditherette_wasm.factory.js';

const module = await WebAssembly.compile(
	await readFile(new URL('../dist/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
async function fresh() {
	const bindings = createScalarBindings();
	const raw = await bindings.default({ module_or_path: module });
	assert.equal(bindings.privateInitialize(1_000_000), 0);
	return { bindings, raw };
}
const input = new Uint8Array([
	128, 128, 128, 0, 128, 128, 128, 127, 128, 128, 128, 128, 128, 128, 128, 255
]);
const controls = [0, 2, 0, 1, 0, 0, 0, 0];
function invoke(bindings, fused = false, sink = {}, policy = controls) {
	const status = fused
		? bindings.privateDitherAndQuantize(
				input,
				2,
				2,
				[0, 0xffffff, 0x1000000],
				0,
				0,
				127,
				0,
				1,
				...policy,
				sink
			)
		: bindings.privatePerturb(input, 2, 2, ...policy, sink);
	if (status !== 0) assert.equal(sink.value, undefined);
	return { status, output: sink.value };
}

test('private field ABI borrows inputs, validates f64 controls and preserves the explicit byte boundary', async () => {
	const glue = await readFile(
		new URL('../dist/scalar/ditherette_wasm.js', import.meta.url),
		'utf8'
	);
	for (const name of ['privatePerturb', 'privateDitherAndQuantize']) {
		const body = glue.match(new RegExp(`export function ${name}\\([^]*?\\n}`))?.[0];
		assert.ok(body);
		assert.doesNotMatch(
			body,
			/__wbindgen_malloc|passArray|\.slice\(|addToExternrefTable|new Uint8Array/
		);
	}
	const { bindings } = await fresh();
	try {
		assert.deepEqual(invoke(bindings), {
			status: 0,
			output: {
				width: 2,
				height: 2,
				data: new Uint8Array([
					104, 104, 104, 0, 136, 136, 136, 127, 152, 152, 152, 128, 120, 120, 120, 255
				])
			}
		});
		assert.deepEqual([...invoke(bindings, true).output.indices], [2, 2, 1, 0]);
		for (const [index, value, path] of [
			[0, 0.5, 18],
			[1, 3, 18],
			[2, 7, 19],
			[3, Infinity, 20],
			[3, 3.5e38, 20],
			[4, 2, 21],
			[5, 1, 21]
		]) {
			const policy = [...controls];
			policy[index] = value;
			for (const fused of [false, true]) {
				assert.equal(invoke(bindings, fused, {}, policy).status, 4);
				assert.equal(bindings.privateErrorPath(), path);
			}
		}
		for (const fused of [false, true]) {
			assert.equal(invoke(bindings, fused, {}, [2, 0, 0, 1, 0, 0, 0, 0]).status, 0);
			for (const parameter of [1, -1, 0.5, NaN, Infinity]) {
				assert.equal(invoke(bindings, fused, {}, [2, parameter, 0, 1, 0, 0, 0, 0]).status, 4);
				assert.equal(bindings.privateErrorPath(), 18);
			}
		}
		assert.equal(invoke(bindings).status, 0);
	} finally {
		bindings.privateDispose();
	}
});

test('512 repeated field failures retain constant handles and memory, shared reentry, recovery and disposal', async () => {
	const { bindings, raw } = await fresh();
	const table = Object.values(raw).find((value) => value instanceof WebAssembly.Table);
	const live = () =>
		Array.from({ length: table.length }, (_, i) => table.get(i)).filter((value) => value !== null)
			.length;
	invoke(bindings);
	invoke(bindings, true);
	const before = { slots: table.length, live: live(), bytes: raw.memory.buffer.byteLength };
	const set = Uint8Array.prototype.set;
	for (let cycle = 0; cycle < 512; cycle++) {
		for (const fused of [false, true]) {
			for (let failAt = 1; failAt <= (fused ? 3 : 2); failAt++) {
				let copies = 0;
				Uint8Array.prototype.set = function (...args) {
					if (++copies === failAt) {
						assert.equal(invoke(bindings).status, 11);
						assert.equal(invoke(bindings, true).status, 11);
						assert.equal(bindings.privateDispose(), 11);
						throw new RangeError('caught copy');
					}
					return Reflect.apply(set, this, args);
				};
				try {
					assert.equal(invoke(bindings, fused).status, 9);
				} finally {
					Uint8Array.prototype.set = set;
				}
				assert.equal(bindings.privateErrorPath(), failAt === 1 ? 4 : 8);
			}
			assert.equal(invoke(bindings, fused, Object.freeze({})).status, 9);
			assert.equal(invoke(bindings, fused).status, 0);
		}
	}
	assert.deepEqual(
		{ slots: table.length, live: live(), bytes: raw.memory.buffer.byteLength },
		before
	);
	bindings.privateDispose();
	bindings.privateDispose();
	assert.equal(invoke(bindings).status, 10);
	assert.equal(invoke(bindings, true).status, 10);
});
