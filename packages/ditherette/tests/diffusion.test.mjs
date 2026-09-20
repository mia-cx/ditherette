import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette, DitheretteError } from '../dist/index.js';

const module = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const vectors = JSON.parse(await readFile(new URL('./fixtures/diffusion.json', import.meta.url)));
const policy = {
	family: 'diffusion',
	kernel: 'floyd-steinberg',
	feedback: 'srgb-bytes',
	strength: 1,
	serpentine: false,
	placement: { mode: 'everywhere' }
};
const request = () => ({
	version: 1,
	source: { width: 2, height: 1, data: new Uint8Array([100, 100, 100, 255, 100, 100, 100, 255]) },
	palette: [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] }
	],
	alpha: { mode: 'preserve', threshold: 0 },
	matching: 'srgb-euclidean',
	dither: policy
});
const errorIs = (code, path) => (error) =>
	error instanceof DitheretteError && error.code === code && error.path === path;

test('public diffusion matches 360 frozen vectors with durable output and unchanged source', async () => {
	const processor = await createDitherette({ wasm: module });
	const backing = new Uint8Array([99, ...vectors.source.data, 99]);
	const source = {
		width: vectors.source.width,
		height: vectors.source.height,
		data: backing.subarray(1, -1)
	};
	const palette = {
		rgba: new Uint8Array(
			vectors.palette.flatMap((entry) =>
				entry.kind === 'transparent' ? [0, 0, 0, 0] : [...entry.rgb, 255]
			)
		),
		transparentIndex: 4
	};
	let first;
	try {
		for (const vector of vectors.cases) {
			const actual = processor.ditherAndQuantize({
				version: 1,
				source,
				palette: vectors.palette,
				alpha: vector.alpha,
				matching: vector.matching,
				dither: vector.dither
			});
			assert.deepEqual(
				actual,
				{
					width: 5,
					height: 7,
					indices: new Uint8Array(vector.indices),
					palette,
					warnings: vector.warnings
				},
				JSON.stringify(vector.dither)
			);
			assert.notEqual(actual.indices.buffer, source.data.buffer);
			if (first) {
				assert.notEqual(actual.indices.buffer, first.indices.buffer);
				assert.notEqual(actual.palette.rgba.buffer, first.palette.rgba.buffer);
			} else first = actual;
		}
		assert.equal(vectors.cases.length, 360);
		assert.deepEqual([...backing], [99, ...vectors.source.data, 99]);
		const retained = structuredClone(first);
		source.data.fill(0);
		processor.ditherAndQuantize(request());
		processor.dispose();
		assert.deepEqual(first, retained);
		assert.throws(() => processor.ditherAndQuantize(request()), errorIs('disposed', 'instance'));
	} finally {
		processor.dispose();
	}
});

test('feedback rounding, transparent sinks, and arithmetic failures preserve their frozen distinctions', async () => {
	const processor = await createDitherette({ wasm: module });
	try {
		for (const [feedback, expected] of [
			['srgb-bytes', [0, 0]],
			['matching', [0, 1]]
		]) {
			const input = request();
			input.source.data = new Uint8Array([1, 0, 0, 255, 1, 0, 0, 255]);
			input.palette[1] = { kind: 'color', rgb: [2, 0, 0] };
			input.dither = { ...policy, feedback };
			assert.deepEqual([...processor.ditherAndQuantize(input).indices], expected);
		}
		for (const [feedback, message] of [
			['srgb-bytes', 'Diffusion work exceeded finite f32 range.'],
			['matching', 'Diffusion matching produced a non-finite distance.']
		]) {
			assert.throws(
				() =>
					processor.ditherAndQuantize({
						...request(),
						dither: { ...policy, feedback, strength: 3.4028234663852886e38 }
					}),
				(error) => errorIs('runtime', 'dither.arithmetic')(error) && error.message === message
			);
			assert.deepEqual([...processor.ditherAndQuantize(request()).indices], [0, 1]);
			const input = request();
			input.source.data[7] = 0;
			input.palette.push({ kind: 'transparent' });
			input.dither = { ...policy, feedback, strength: 3.4028234663852886e38 };
			assert.deepEqual([...processor.ditherAndQuantize(input).indices], [0, 2]);
		}
		const input = request();
		input.palette = Array.from({ length: 257 }, () => ({ kind: 'transparent' }));
		const transparent = processor.ditherAndQuantize(input);
		assert.deepEqual([...transparent.indices], [0, 0]);
		assert.equal(transparent.palette.transparentIndex, 0);
		assert.deepEqual(
			transparent.warnings.map((w) => w.code),
			['palette-truncated', 'transparent-only']
		);
	} finally {
		processor.dispose();
	}
});

test('invalid diffusion controls fail at their paths and leave the processor usable', async () => {
	const processor = await createDitherette({ wasm: module });
	try {
		for (const [change, path] of [
			[{ kernel: 'unknown' }, 'kernel'],
			[{ feedback: 'linear' }, 'feedback'],
			[{ serpentine: 1 }, 'serpentine'],
			[{ strength: NaN }, 'strength'],
			[{ strength: Infinity }, 'strength'],
			[{ strength: -1 }, 'strength'],
			[{ strength: 3.5e38 }, 'strength'],
			[{ perturb: {} }, 'perturb'],
			[
				{ placement: { mode: 'adaptive', radius: 0, threshold: 0, softness: 0 } },
				'placement.radius'
			],
			[
				{ placement: { mode: 'adaptive', radius: 1, threshold: -1, softness: 0 } },
				'placement.threshold'
			],
			[
				{ placement: { mode: 'adaptive', radius: 1, threshold: 0, softness: Infinity } },
				'placement.softness'
			]
		]) {
			assert.throws(
				() => processor.ditherAndQuantize({ ...request(), dither: { ...policy, ...change } }),
				errorIs('invalid-settings', `dither.${path}`)
			);
			assert.deepEqual([...processor.ditherAndQuantize(request()).indices], [0, 1]);
		}
	} finally {
		processor.dispose();
	}
});

test('public diffusion enforces the exact budget before output and recovers from caught copy failures', async () => {
	async function succeeds(memoryLimitBytes) {
		let processor;
		try {
			processor = await createDitherette({ wasm: module, memoryLimitBytes });
			processor.ditherAndQuantize(request());
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
	const under = await createDitherette({ wasm: module, memoryLimitBytes: low - 1 });
	const originalSet = Uint8Array.prototype.set;
	let copies = 0;
	Uint8Array.prototype.set = function (...args) {
		copies++;
		return Reflect.apply(originalSet, this, args);
	};
	try {
		assert.throws(
			() => under.ditherAndQuantize(request()),
			errorIs('memory-limit', 'memoryLimitBytes')
		);
	} finally {
		Uint8Array.prototype.set = originalSet;
		under.dispose();
	}
	assert.ok(copies <= 1, 'Only the input snapshot may precede remaining preflight');
	const processor = await createDitherette({ wasm: module, memoryLimitBytes: low });
	const expected = processor.ditherAndQuantize(request());
	try {
		for (let failAt = 1; failAt <= 3; failAt++) {
			let copy = 0;
			Uint8Array.prototype.set = function (...args) {
				if (++copy === failAt) throw new RangeError('copy');
				return Reflect.apply(originalSet, this, args);
			};
			try {
				assert.throws(
					() => processor.ditherAndQuantize(request()),
					errorIs('wasm-memory-unavailable', failAt === 1 ? 'source.data' : 'output')
				);
			} finally {
				Uint8Array.prototype.set = originalSet;
			}
			assert.deepEqual(processor.ditherAndQuantize(request()), expected);
		}
	} finally {
		processor.dispose();
	}
});
