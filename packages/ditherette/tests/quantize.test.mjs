import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette, DitheretteError } from '../dist/index.js';

// Development fixture host only. Installed browser coverage uses the same public request shape.
const module = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const color = (...rgb) => ({ kind: 'color', rgb });
const request = () => ({
	version: 1,
	source: { width: 2, height: 1, data: new Uint8Array([255, 0, 0, 128, 17, 31, 53, 0]) },
	palette: [color(255, 0, 0), color(0, 0, 0), { kind: 'transparent' }],
	alpha: { mode: 'preserve', threshold: 127.9999999 },
	matching: 'srgb-euclidean'
});
const errorIs = (code, path) => (error) =>
	error instanceof DitheretteError && error.code === code && error.path === path;

test('warm preparation validates palette tails and preserves order, thresholds, and durable metadata', async () => {
	const processor = await createDitherette({ wasm: module });
	try {
		const value = request();
		value.palette = [...Array.from({ length: 256 }, () => color(0, 0, 0)), color(255, 255, 255)];
		const first = processor.quantize(value);
		value.palette[256] = { kind: 'color', rgb: [256, 0, 0] };
		assert.throws(() => processor.quantize(value), errorIs('invalid-palette', 'palette.256.rgb.0'));
		value.palette[256] = { kind: 'transparent' };
		assert.deepEqual(processor.quantize(value), first);
		value.palette = value.palette.slice(0, 256);
		assert.equal(
			processor.quantize(value).warnings.some(({ code }) => code === 'palette-truncated'),
			false
		);
		const precise = request();
		assert.deepEqual([...processor.quantize(precise).indices], [0, 2]);
		precise.alpha.threshold = 128.0000001;
		assert.deepEqual([...processor.quantize(precise).indices], [2, 2]);
		precise.alpha.threshold = -0;
		const zero = processor.quantize(precise);
		precise.alpha.threshold = 0;
		assert.deepEqual(processor.quantize(precise), zero);
		precise.palette = [...precise.palette].reverse();
		assert.deepEqual([...processor.quantize(precise).indices], [2, 0]);
		assert.equal(first.warnings[0].code, 'palette-truncated');
	} finally {
		processor.dispose();
	}
});

test('weighted public tags select independently calculated winners', async () => {
	const processor = await createDitherette({ wasm: module });
	try {
		for (const [matching, rgbs, expected] of [
			[
				'srgb-compuphase',
				[
					[17, 0, 0],
					[0, 0, 14]
				],
				1
			],
			[
				'srgb-rec601',
				[
					[15, 0, 0],
					[0, 0, 25]
				],
				0
			],
			[
				'srgb-rec709',
				[
					[15, 0, 0],
					[0, 0, 25]
				],
				1
			]
		]) {
			const output = processor.quantize({
				...request(),
				matching,
				source: { width: 1, height: 1, data: new Uint8Array([0, 0, 0, 255]) },
				palette: rgbs.map((rgb) => color(...rgb))
			});
			assert.deepEqual([...output.indices], [expected]);
		}
	} finally {
		processor.dispose();
	}
});

test('quantize returns exact fifteen-pair indices and durable independent palette metadata', async (t) => {
	const memories = [];
	const instantiate = WebAssembly.instantiate;
	t.mock.method(WebAssembly, 'instantiate', async (...args) => {
		const instance = await instantiate(...args);
		memories.push(
			(instance instanceof WebAssembly.Instance ? instance : instance.instance).exports.memory
		);
		return instance;
	});
	const [processor, other] = await Promise.all([
		createDitherette({ wasm: module }),
		createDitherette({ wasm: module })
	]);
	const backing = new Uint8Array([9, 255, 0, 0, 128, 17, 31, 53, 0, 9]);
	const value = request();
	value.source.data = backing.subarray(1, 9);
	let previous;
	for (const matching of [
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
	]) {
		const output = processor.quantize({ ...value, matching });
		assert.deepEqual(output, {
			width: 2,
			height: 1,
			indices: new Uint8Array([0, 2]),
			palette: {
				rgba: new Uint8Array([255, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 0]),
				transparentIndex: 2
			},
			warnings: []
		});
		assert.notEqual(output.indices.buffer, value.source.data.buffer);
		assert.notEqual(output.indices.buffer, output.palette.rgba.buffer);
		if (previous) assert.notEqual(output.palette.rgba.buffer, previous.palette.rgba.buffer);
		previous = output;
	}
	memories[0].grow(1);
	value.palette[0].rgb[0] = 12;
	value.source.data[0] = 1;
	processor.quantize(value);
	assert.deepEqual([...previous.indices], [0, 2]);
	assert.equal(previous.palette.rgba[0], 255);
	assert.deepEqual([...backing], [9, 1, 0, 0, 128, 17, 31, 53, 0, 9]);
	processor.dispose();
	processor.dispose();
	assert.throws(() => processor.quantize(request()), errorIs('disposed', 'instance'));
	assert.deepEqual([...other.quantize(request()).indices], [0, 2]);
	other.dispose();
	assert.deepEqual([...previous.indices], [0, 2]);
});

test('quantize retains alpha rounding, first ties, transparency and exact warning text', async () => {
	const processor = await createDitherette({ wasm: module });
	for (const [alpha, rgb] of [
		[{ mode: 'premultiplied' }, [128, 0, 0]],
		[{ mode: 'matte', rgb: [255, 255, 255] }, [255, 127, 127]]
	]) {
		const value = request();
		value.source = { width: 1, height: 1, data: value.source.data.subarray(0, 4) };
		value.alpha = alpha;
		value.palette = [color(...rgb), color(...rgb), color(255, 0, 0)];
		assert.deepEqual([...processor.quantize(value).indices], [0]);
	}
	const truncated = processor.quantize({
		...request(),
		palette: [...Array.from({ length: 256 }, () => color(0, 0, 0)), { kind: 'transparent' }]
	});
	assert.equal(truncated.palette.rgba.length, 1024);
	assert.equal(truncated.palette.transparentIndex, null);
	assert.deepEqual(truncated.warnings, [
		{
			code: 'palette-truncated',
			message: 'Palette was truncated to 256 entries for indexed PNG export.'
		},
		{
			code: 'transparent-fallback',
			message:
				'Transparent is disabled; alpha-thresholded pixels use the darkest enabled visible color.'
		}
	]);
	const transparent = processor.quantize({
		...request(),
		palette: [{ kind: 'transparent' }, { kind: 'transparent' }]
	});
	assert.deepEqual([...transparent.indices], [0, 0]);
	assert.equal(transparent.palette.transparentIndex, 0);
	assert.deepEqual(transparent.warnings, [
		{
			code: 'transparent-only',
			message: 'Only Transparent is enabled; every output pixel is transparent.'
		}
	]);
	processor.dispose();
});

test('quantize rejects malformed settings and tail entries before copying source', async () => {
	const processor = await createDitherette({ wasm: module });
	for (const [change, code, path] of [
		[{ matching: { 'srgb-euclidean': null } }, 'invalid-settings', 'matching'],
		[{ matching: 'oklch-ciede2000' }, 'invalid-settings', 'matching'],
		[{ alpha: { mode: 'preserve', threshold: NaN } }, 'invalid-settings', 'alpha.threshold'],
		[{ alpha: { mode: 'premultiplied', threshold: 0 } }, 'invalid-settings', 'alpha.threshold'],
		[{ palette: [{ kind: 'transparent', rgb: [0, 0, 0] }] }, 'invalid-palette', 'palette.0.rgb'],
		[{ palette: [color(0, 0, 256)] }, 'invalid-palette', 'palette.0.rgb.2'],
		[{ palette: [] }, 'invalid-palette', 'palette'],
		[
			{ palette: [...Array.from({ length: 256 }, () => color(0, 0, 0)), { kind: 'nope' }] },
			'invalid-palette',
			'palette.256.kind'
		],
		[{ onProgress: 1 }, 'invalid-settings', 'onProgress']
	])
		assert.throws(() => processor.quantize({ ...request(), ...change }), errorIs(code, path));
	const detached = request();
	structuredClone(detached.source.data.buffer, { transfer: [detached.source.data.buffer] });
	assert.throws(() => processor.quantize(detached), errorIs('invalid-image', 'source.data'));
	const recursive = request();
	Object.defineProperty(recursive, 'palette', {
		get() {
			assert.throws(() => processor.quantize(request()), errorIs('reentrant-call', 'instance'));
			assert.throws(() => processor.dispose(), errorIs('reentrant-call', 'instance'));
			return request().palette;
		}
	});
	assert.deepEqual([...processor.quantize(recursive).indices], [0, 2]);
	processor.dispose();
});

test('quantize exact budget and one-under preserve the public allocation failure contract', async () => {
	async function succeeds(memoryLimitBytes) {
		let processor;
		try {
			processor = await createDitherette({ wasm: module, memoryLimitBytes });
			processor.quantize(request());
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
	let copies = 0;
	const set = Uint8Array.prototype.set;
	Uint8Array.prototype.set = function (...args) {
		copies++;
		return Reflect.apply(set, this, args);
	};
	try {
		assert.throws(() => under.quantize(request()), errorIs('memory-limit', 'memoryLimitBytes'));
	} finally {
		Uint8Array.prototype.set = set;
		under.dispose();
	}
	assert.ok(copies <= 1, 'Only the input snapshot may precede remaining preflight');
	const processor = await createDitherette({ wasm: module, memoryLimitBytes: low });
	for (const failAt of [1, 2, 3]) {
		let copy = 0;
		Uint8Array.prototype.set = function (...args) {
			if (++copy === failAt) throw new RangeError('copy');
			return Reflect.apply(set, this, args);
		};
		try {
			assert.throws(
				() => processor.quantize(request()),
				errorIs('wasm-memory-unavailable', failAt === 1 ? 'source.data' : 'output')
			);
		} finally {
			Uint8Array.prototype.set = set;
		}
		assert.deepEqual([...processor.quantize(request()).indices], [0, 2]);
	}
	processor.dispose();
});
