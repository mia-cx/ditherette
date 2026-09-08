import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette, DitheretteError } from '../dist/index.js';

const module = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const vectors = JSON.parse(await readFile(new URL('./fixtures/fields.json', import.meta.url)));
const request = () => ({
	version: 1,
	source: { ...vectors.source, data: new Uint8Array(vectors.source.data) },
	perturb: vectors.cases[0].policy
});
const quantize = () => ({
	version: 1,
	source: request().source,
	palette: [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'transparent' }
	],
	alpha: { mode: 'preserve', threshold: 127.9999999 },
	matching: 'srgb-euclidean'
});
const fused = () => ({
	...quantize(),
	dither: { family: 'separable', perturb: request().perturb }
});
const errorIs = (code, path) => (error) =>
	error instanceof DitheretteError && error.code === code && error.path === path;

test('public fields match frozen vectors and all matching pairs consume the RGBA8 boundary', async () => {
	const processor = await createDitherette({ wasm: module });
	const backing = new Uint8Array([99, ...vectors.source.data, 99]);
	const source = { ...vectors.source, data: backing.subarray(1, 17) };
	const modes = [
		'srgb-euclidean',
		'linear-rgb-euclidean',
		'oklab-euclidean',
		'cielab-euclidean',
		'ycbcr-euclidean',
		'srgb-compuphase',
		'srgb-rec601',
		'srgb-rec709',
		'oklch-euclidean',
		'oklch-circular-hue',
		'oklch-hue-arc',
		'cielab-ciede2000',
		'cielch-euclidean',
		'cielch-circular-hue',
		'cielch-hue-arc'
	];
	let previous;
	try {
		for (const vector of vectors.cases) {
			const vectorSource = vector.source
				? { ...vector.source, data: new Uint8Array(vector.source.data) }
				: source;
			const rgba = processor.perturb({ version: 1, source: vectorSource, perturb: vector.policy });
			assert.deepEqual([...rgba.data], vector.rgba, JSON.stringify(vector.policy));
			assert.notEqual(rgba.data.buffer, source.data.buffer);
			if (previous) assert.notEqual(rgba.data.buffer, previous.data.buffer);
			for (const matching of modes) {
				const value = { ...quantize(), source: vectorSource, matching };
				assert.deepEqual(
					processor.ditherAndQuantize({
						...value,
						dither: { family: 'separable', perturb: vector.policy }
					}),
					processor.quantize({ ...value, source: rgba })
				);
			}
			previous = rgba;
		}
		for (const space of ['srgb', 'linear-rgb', 'oklab', 'oklch', 'cielab', 'cielch', 'ycbcr']) {
			assert.deepEqual(
				processor.perturb({ ...request(), perturb: { ...request().perturb, space, strength: 0 } })
					.data,
				source.data
			);
		}
		assert.deepEqual(
			processor.ditherAndQuantize({ ...quantize(), dither: { family: 'none' } }),
			processor.quantize(quantize())
		);
		assert.deepEqual([...backing], [99, ...vectors.source.data, 99]);
		const retained = [...previous.data];
		source.data.fill(0);
		processor.perturb(request());
		processor.dispose();
		assert.deepEqual([...previous.data], retained);
	} finally {
		processor.dispose();
	}
});

test('field validation retains canonical tags, nested paths, shared reentry and disposal', async () => {
	const processor = await createDitherette({ wasm: module });
	try {
		for (const [change, code, path] of [
			[{ field: { algorithm: 'bayer', size: 2 } }, 'invalid-settings', 'field.size'],
			[{ field: { algorithm: 'bayer', size: '2', seed: 0 } }, 'invalid-settings', 'field.seed'],
			[{ field: { algorithm: 'random', seed: -1 } }, 'invalid-settings', 'field.seed'],
			[{ field: { algorithm: 'blue-noise', seed: 0 } }, 'invalid-settings', 'field.seed'],
			[{ field: { algorithm: 'blue-noise', size: '32' } }, 'invalid-settings', 'field.size'],
			[{ space: 'srgb-rec709' }, 'invalid-settings', 'space'],
			[{ strength: Infinity }, 'invalid-settings', 'strength'],
			[{ strength: 3.5e38 }, 'invalid-settings', 'strength'],
			[{ placement: { mode: 'everywhere', radius: 1 } }, 'invalid-settings', 'placement.radius'],
			[
				{ placement: { mode: 'adaptive', radius: 0, threshold: 0, softness: 0 } },
				'invalid-settings',
				'placement.radius'
			]
		]) {
			const perturb = { ...request().perturb, ...change };
			assert.throws(
				() => processor.perturb({ ...request(), perturb }),
				errorIs(code, `perturb.${path}`)
			);
			assert.throws(
				() => processor.ditherAndQuantize({ ...fused(), dither: { family: 'separable', perturb } }),
				errorIs(code, `dither.perturb.${path}`)
			);
		}
		assert.throws(
			() => processor.perturb({ ...request(), onProgress() {} }),
			errorIs('unsupported-operation', 'onProgress')
		);
		assert.throws(
			() => processor.ditherAndQuantize({ ...fused(), dither: { family: 'diffusion' } }),
			errorIs('unsupported-operation', 'dither.family')
		);
		const recursive = request();
		Object.defineProperty(recursive, 'perturb', {
			get() {
				for (const action of [
					() => processor.perturb(request()),
					() => processor.ditherAndQuantize(fused()),
					() => processor.quantize(quantize()),
					() => processor.dispose()
				])
					assert.throws(action, errorIs('reentrant-call', 'instance'));
				return request().perturb;
			}
		});
		assert.deepEqual([...processor.perturb(recursive).data], vectors.cases[0].rgba);
	} finally {
		processor.dispose();
	}
	assert.throws(() => processor.perturb(request()), errorIs('disposed', 'instance'));
	assert.throws(() => processor.ditherAndQuantize(fused()), errorIs('disposed', 'instance'));
});

test('both field result shapes enforce exact budgets before copying and recover from each caught copy failure', async () => {
	for (const [method, value, count] of [
		['perturb', request, 2],
		['ditherAndQuantize', fused, 3]
	]) {
		async function succeeds(memoryLimitBytes) {
			let processor;
			try {
				processor = await createDitherette({ wasm: module, memoryLimitBytes });
				processor[method](value());
				return true;
			} catch (error) {
				assert.equal(error.code, 'memory-limit');
				return false;
			} finally {
				processor?.dispose();
			}
		}
		let low = 1,
			high = 100_000;
		while (low < high) {
			const middle = Math.floor((low + high) / 2);
			if (await succeeds(middle)) high = middle;
			else low = middle + 1;
		}
		assert.ok(await succeeds(low));
		const under = await createDitherette({ wasm: module, memoryLimitBytes: low - 1 });
		const set = Uint8Array.prototype.set;
		let copies = 0;
		Uint8Array.prototype.set = function (...args) {
			copies++;
			return Reflect.apply(set, this, args);
		};
		try {
			assert.throws(() => under[method](value()), errorIs('memory-limit', 'memoryLimitBytes'));
		} finally {
			Uint8Array.prototype.set = set;
			under.dispose();
		}
		assert.equal(copies, 0);
		const processor = await createDitherette({ wasm: module, memoryLimitBytes: low });
		const expected = processor[method](value());
		try {
			for (let failAt = 1; failAt <= count; failAt++) {
				let copy = 0;
				Uint8Array.prototype.set = function (...args) {
					if (++copy === failAt) throw new RangeError('copy');
					return Reflect.apply(set, this, args);
				};
				try {
					assert.throws(
						() => processor[method](value()),
						errorIs('wasm-memory-unavailable', failAt === 1 ? 'source.data' : 'output')
					);
				} finally {
					Uint8Array.prototype.set = set;
				}
				assert.deepEqual(processor[method](value()), expected);
			}
		} finally {
			processor.dispose();
		}
	}
});
