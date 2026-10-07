import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette, isEffect } from '../dist/index.js';

const wasm = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);

/** Every byte on each channel, with varied alpha including zero. */
function ramp() {
	const data = new Uint8Array(256 * 4);
	for (let value = 0; value < 256; value++)
		data.set([value, 255 - value, (value * 7) & 255, (value * 3) & 255], value * 4);
	return { width: 16, height: 16, data };
}

/** The ramp at full opacity: recipe v2 resizes it exactly like v1. */
function opaqueRamp() {
	const source = ramp();
	for (let offset = 3; offset < source.data.length; offset += 4) source.data[offset] = 255;
	return source;
}

const levels = (input, gamma, output, channel = 'rgb', enabled = true) => ({
	effect: 'levels',
	enabled,
	channel,
	input: { black: input[0], white: input[1] },
	gamma,
	output: { black: output[0], white: output[1] }
});
const halve = levels([0, 1], 1, [0, 0.5]);
const double = levels([0, 0.5], 1, [0, 1]);
const palette = [
	{ kind: 'color', rgb: [0, 0, 0] },
	{ kind: 'transparent' },
	{ kind: 'color', rgb: [200, 120, 40] },
	{ kind: 'color', rgb: [255, 255, 255] }
];
const terminal = {
	output: { width: 7, height: 5, resize: { algorithm: 'area' } },
	alpha: { mode: 'preserve', threshold: 127.5 },
	match: 'oklab-euclidean',
	dither: {
		family: 'diffusion',
		kernel: 'floyd-steinberg',
		strength: 1,
		placement: { mode: 'everywhere' },
		serpentine: true,
		feedback: 'matching'
	}
};

async function withProcessor(run) {
	const processor = await createDitherette({ wasm });
	try {
		await run(processor);
	} finally {
		processor.dispose();
	}
}

const apply = (processor, effects, extra = {}) =>
	processor.applyEffects({ version: 1, source: ramp(), effects, ...extra });

test('empty and disabled chains return the caller source itself', () =>
	withProcessor((processor) => {
		for (const effects of [[], [levels([0.2, 0.4], 3, [1, 0], 'rgb', false)]]) {
			const source = ramp();
			const stages = [];
			const result = processor.applyEffects({
				version: 1,
				source,
				effects,
				onProgress: ({ stage }) => stages.push(stage)
			});
			assert.equal(result, source);
			assert.deepEqual(stages, ['prepare', 'complete']);
		}
	}));

test('chains run in caller order on continuous colour and keep alpha', () =>
	withProcessor((processor) => {
		const source = ramp();
		assert.deepEqual(apply(processor, [halve, double]).data, source.data);
		const staged = apply(processor, [double], { source: apply(processor, [halve]) });
		assert.notDeepEqual(staged.data, source.data, 'rounding between calls loses odd values');
		const compress = levels([0, 1], 1, [0.25, 0.75]);
		assert.notDeepEqual(
			apply(processor, [double, compress]).data,
			apply(processor, [compress, double]).data
		);
		const repeated = apply(processor, [
			levels([0, 1], 2, [0, 1], 'blue'),
			levels([0, 1], 1, [0.5, 1], 'blue')
		]);
		for (let index = 0; index < source.data.length; index += 4) {
			assert.deepEqual(
				repeated.data.subarray(index, index + 2),
				source.data.subarray(index, index + 2)
			);
			assert.equal(repeated.data[index + 3], source.data[index + 3]);
		}
	}));

test('results are independent of the source and repeat exactly', () =>
	withProcessor((processor) => {
		const source = ramp();
		const effects = [levels([0.1, 0.9], 1.4, [0, 1]), levels([0, 1], 1, [0.1, 0.9], 'red')];
		const first = processor.applyEffects({ version: 1, source, effects });
		const copy = first.data.slice();
		source.data.fill(0);
		assert.deepEqual(first.data, copy);
		const second = processor.applyEffects({ version: 1, source: ramp(), effects });
		assert.deepEqual(second.data, copy);
		assert.notEqual(second.data, first.data);
	}));

test('opaque process v2 equals applyEffects then process v1, with an effects progress stage', () =>
	withProcessor((processor) => {
		const effects = [levels([0.1, 0.8], 1.3, [0, 1]), levels([0, 1], 1, [0.3, 0.6], 'blue')];
		const stages = [];
		const composed = processor.process({
			source: opaqueRamp(),
			palette,
			recipe: { version: 2, effects, ...terminal },
			onProgress: ({ stage }) => stages.push(stage)
		});
		const staged = processor.process({
			source: apply(processor, effects, { source: opaqueRamp() }),
			palette,
			recipe: { version: 1, ...terminal }
		});
		assert.deepEqual(composed, staged);
		assert.equal(stages[0], 'prepare');
		assert.ok(stages.indexOf('effects') > 0, stages.join());
		assert.ok(stages.indexOf('effects') < stages.indexOf('resize'), stages.join());
		assert.equal(stages.at(-1), 'complete');
		const plain = processor.process({
			source: opaqueRamp(),
			palette,
			recipe: { version: 2, effects: [], ...terminal }
		});
		assert.deepEqual(
			plain,
			processor.process({ source: opaqueRamp(), palette, recipe: { version: 1, ...terminal } })
		);
	}));

test('process v2 keeps colour hidden under transparent pixels out of filtered resizes', () =>
	withProcessor((processor) => {
		const hiding = (rgb) => {
			const source = ramp();
			for (let offset = 0; offset < source.data.length; offset += 4)
				if (source.data[offset + 3] < 64) source.data.set([...rgb, 0], offset);
			return source;
		};
		const run = (recipe, rgb) => processor.process({ source: hiding(rgb), palette, recipe });
		const v2 = { version: 2, effects: [], ...terminal };
		const v1 = { version: 1, ...terminal };
		assert.deepEqual(run(v2, [0, 0, 0]), run(v2, [255, 255, 255]));
		assert.notDeepEqual(run(v1, [0, 0, 0]), run(v1, [255, 255, 255]));
	}));

test('invalid effects fail with indexed paths before any work', () =>
	withProcessor((processor) => {
		const fails = (request, code, path) =>
			assert.throws(
				() => processor.applyEffects(request),
				(error) => error.code === code && error.path === path,
				`${code} ${path}`
			);
		const base = { version: 1, source: ramp() };
		fails(
			{ ...base, effects: [halve, { ...halve, effect: 'blur' }] },
			'invalid-settings',
			'effects.1.effect'
		);
		fails({ ...base, effects: [{ ...halve, gama: 1 }] }, 'invalid-settings', 'effects.0.gama');
		fails(
			{ ...base, effects: [{ ...halve, enabled: 'yes' }] },
			'invalid-settings',
			'effects.0.enabled'
		);
		fails(
			{ ...base, effects: [levels([0.5, 0.5], 1, [0, 1], 'rgb', false)] },
			'invalid-settings',
			'effects.0.input'
		);
		fails(
			{ ...base, effects: [levels([0, 1], 20, [0, 1])] },
			'invalid-settings',
			'effects.0.gamma'
		);
		fails(
			{ ...base, effects: [levels([0, 1], 1, [0, Number.NaN])] },
			'invalid-settings',
			'effects.0.output.white'
		);
		fails(
			{ ...base, effects: [levels([0, 1], 1, [0, 1], 'luma')] },
			'invalid-settings',
			'effects.0.channel'
		);
		fails({ ...base, effects: {} }, 'invalid-settings', 'effects');
		fails({ ...base, effects: Array(65).fill(halve) }, 'invalid-settings', 'effects');
		fails({ ...base, effects: [], context: { space: 'hsv' } }, 'invalid-settings', 'context.space');
		assert.deepEqual(
			processor.applyEffects({ ...base, effects: [halve, double], context: { palette: [] } }).data,
			ramp().data,
			'an empty palette is no palette'
		);
		fails(
			{ ...base, effects: [], context: { palette: [{ kind: 'colour' }] } },
			'invalid-palette',
			'context.palette.0.kind'
		);
		fails({ ...base, effects: [], context: { mood: 1 } }, 'invalid-settings', 'context.mood');
		fails({ ...base, version: 2, effects: [] }, 'invalid-request', 'version');
		assert.throws(
			() =>
				processor.process({
					source: ramp(),
					palette,
					recipe: { version: 2, effects: [levels([0, 1], 0, [0, 1])], ...terminal }
				}),
			(error) => error.code === 'invalid-settings' && error.path === 'recipe.effects.0.gamma'
		);
		assert.throws(
			() =>
				processor.process({
					source: ramp(),
					palette,
					recipe: { version: 1, effects: [], ...terminal }
				}),
			(error) => error.path === 'recipe.effects'
		);
		assert.deepEqual(apply(processor, [halve, double]).data, ramp().data, 'instance stays usable');
	}));

const twoInputCurve = {
	kind: 'adjust',
	x: { model: 'hsl', channel: 'hue' },
	x2: { model: 'oklch', channel: 'lightness' },
	y: { model: 'cielch', channel: 'hue' },
	grid: {
		columns: [0.1, 0.55],
		rows: [0, 0.5, 1],
		values: [
			[0.5, 0.8],
			[0.2, 0.5],
			[0.7, 0.3]
		]
	}
};

const grading = {
	curves: {
		effect: 'curves',
		enabled: true,
		curves: [
			{
				kind: 'remap',
				x: { model: 'srgb', channel: 'green' },
				y: { model: 'srgb', channel: 'green' },
				points: [
					[0, 0],
					[0.3, 0.2],
					[0.7, 0.85],
					[1, 1]
				]
			},
			{
				kind: 'adjust',
				x: { model: 'srgb', channel: 'green' },
				y: { model: 'srgb', channel: 'red' },
				points: [
					[0, 0.25],
					[1, 0.75]
				]
			}
		]
	},
	'brightness-contrast': {
		effect: 'brightness-contrast',
		enabled: true,
		brightness: 0.05,
		contrast: 0.3
	},
	exposure: { effect: 'exposure', enabled: true, stops: 0.5 },
	'white-balance': { effect: 'white-balance', enabled: true, temperature: 0.4, tint: -0.2 },
	'hue-saturation': {
		effect: 'hue-saturation',
		enabled: true,
		hue: 40,
		saturation: 0.3,
		lightness: 0.1
	}
};
const neutral = {
	curves: {
		...grading.curves,
		curves: [
			{
				kind: 'remap',
				x: { model: 'hsl', channel: 'hue' },
				y: { model: 'hsl', channel: 'hue' },
				points: [
					[0, 0],
					[1, 1]
				]
			}
		]
	},
	'brightness-contrast': { ...grading['brightness-contrast'], brightness: 0, contrast: 0 },
	exposure: { ...grading.exposure, stops: 0 },
	'white-balance': { ...grading['white-balance'], temperature: 0, tint: 0 },
	'hue-saturation': { ...grading['hue-saturation'], hue: 0, saturation: 0, lightness: 0 }
};

test('every grading effect runs, keeps alpha, and is exact when neutral', () =>
	withProcessor((processor) => {
		const source = ramp();
		for (const [name, effect] of Object.entries(grading)) {
			const graded = apply(processor, [effect]);
			assert.notDeepEqual(graded.data, source.data, name);
			for (let index = 3; index < source.data.length; index += 4)
				assert.equal(graded.data[index], source.data[index], name);
			assert.deepEqual(apply(processor, [neutral[name]]).data, source.data, `${name} neutral`);
		}
		const chain = Object.values(grading);
		const composed = processor.process({
			source: ramp(),
			palette,
			recipe: { version: 2, effects: chain, ...terminal }
		});
		const staged = processor.process({
			source: apply(processor, chain),
			palette,
			recipe: { version: 1, ...terminal }
		});
		assert.deepEqual(composed, staged);
	}));

test('two-input curves run and preserve alpha', () =>
	withProcessor((processor) => {
		const source = ramp();
		const graded = apply(processor, [{ effect: 'curves', enabled: true, curves: [twoInputCurve] }]);
		assert.notDeepEqual(graded.data, source.data);
		for (let index = 3; index < source.data.length; index += 4)
			assert.equal(graded.data[index], source.data[index]);
	}));

test('grading arguments are validated with indexed paths', () =>
	withProcessor((processor) => {
		const fails = (effect, path) =>
			assert.throws(
				() => apply(processor, [halve, effect]),
				(error) => error.code === 'invalid-settings' && error.path === path,
				path
			);
		const curve = grading.curves.curves[0];
		const gridCurve = twoInputCurve;
		const withCurve = (changed) => ({ ...grading.curves, curves: [changed] });
		fails({ ...grading.curves, curves: Array(17).fill(curve) }, 'effects.1.curves');
		fails(withCurve({ ...curve, kind: 'blend' }), 'effects.1.curves.0.kind');
		fails(
			withCurve({ ...curve, x: { model: 'hsv', channel: 'lightness' } }),
			'effects.1.curves.0.x.channel'
		);
		fails(
			withCurve({ ...curve, y: { model: 'unknown', channel: 'red' } }),
			'effects.1.curves.0.y.model'
		);
		fails(withCurve({ ...curve, y: { model: 'srgb', channel: 'red' } }), 'effects.1.curves.0.y');
		fails(withCurve({ ...curve, points: [[0, 0]] }), 'effects.1.curves.0.points');
		fails(
			{
				...grading.curves,
				curves: [
					curve,
					{
						...curve,
						points: [
							[0, 0],
							[0.5, 1],
							[0.5, 1]
						]
					}
				]
			},
			'effects.1.curves.1.points.2.0'
		);
		fails(
			withCurve({
				...curve,
				points: [
					[0, 0],
					[1, 2]
				]
			}),
			'effects.1.curves.0.points.1.1'
		);
		fails(
			withCurve({
				...curve,
				points: [
					[0, 0],
					[1e-25, 0.5],
					[1, 1]
				]
			}),
			'effects.1.curves.0.points.1.0'
		);
		fails(withCurve({ ...curve, points: [[0, 0], [1]] }), 'effects.1.curves.0.points.1');
		fails(
			withCurve({
				...curve,
				kind: 'adjust',
				x: { model: 'hsl', channel: 'hue' },
				y: { model: 'hsl', channel: 'hue' },
				points: [
					[0.1, 0.5],
					[1, 0.5]
				]
			}),
			'effects.1.curves.0.points.0.0'
		);
		fails(
			withCurve({
				...curve,
				kind: 'adjust',
				x: { model: 'hsl', channel: 'hue' },
				y: { model: 'hsl', channel: 'hue' },
				points: [
					[0, 0.5],
					[0.5, 0.8],
					[0.9, 0.5]
				]
			}),
			'effects.1.curves.0.points.2.0'
		);
		fails(
			withCurve({
				...curve,
				kind: 'adjust',
				x: { model: 'hsl', channel: 'hue' },
				points: [
					[0, 0.5],
					[0.5, 0.8],
					[1, 0.6]
				]
			}),
			'effects.1.curves.0.points.2.1'
		);
		fails(withCurve({ ...curve, mode: 'absolute' }), 'effects.1.curves.0.mode');
		fails(withCurve({ ...gridCurve, points: curve.points }), 'effects.1.curves.0.points');
		fails(
			withCurve({
				...gridCurve,
				kind: 'remap',
				x: { model: 'hsv', channel: 'lightness' }
			}),
			'effects.1.curves.0.kind'
		);
		fails(
			withCurve({
				...gridCurve,
				x2: { model: 'hsv', channel: 'lightness' },
				y: { model: 'unknown', channel: 'red' }
			}),
			'effects.1.curves.0.x2.channel'
		);
		fails(withCurve({ ...gridCurve, x2: gridCurve.x }), 'effects.1.curves.0.x2');
		fails(
			withCurve({ ...gridCurve, y: { model: 'unknown', channel: 'red' }, grid: null }),
			'effects.1.curves.0.y.model'
		);
		fails(
			withCurve({ ...gridCurve, grid: { ...gridCurve.grid, rows: [0, , 1] } }),
			'effects.1.curves.0.grid.rows.1'
		);
		fails(
			withCurve({
				...gridCurve,
				grid: { ...gridCurve.grid, values: [[0.5], ...gridCurve.grid.values.slice(1)] }
			}),
			'effects.1.curves.0.grid.values.0'
		);
		fails(
			withCurve({
				...gridCurve,
				grid: { ...gridCurve.grid, values: gridCurve.grid.values.slice(1) }
			}),
			'effects.1.curves.0.grid.values'
		);
		fails(
			withCurve({
				...gridCurve,
				grid: {
					...gridCurve.grid,
					values: [[1.0001, 0.5], ...gridCurve.grid.values.slice(1)]
				}
			}),
			'effects.1.curves.0.grid.values.0.0'
		);
		fails(
			withCurve({ ...gridCurve, grid: { ...gridCurve.grid, extra: true } }),
			'effects.1.curves.0.grid.extra'
		);
		fails(
			withCurve({
				...gridCurve,
				grid: {
					...gridCurve.grid,
					columns: [0, 0.9995],
					values: gridCurve.grid.values.map(() => [0.5, 0.5])
				}
			}),
			'effects.1.curves.0.grid.columns.0'
		);
		fails(
			withCurve({
				...gridCurve,
				grid: {
					...gridCurve.grid,
					columns: Array.from({ length: 49 }, (_, index) => index / 50),
					values: gridCurve.grid.values.map(() => Array(49).fill(0.5))
				}
			}),
			'effects.1.curves.0.grid.columns'
		);
		fails(
			withCurve({
				...gridCurve,
				grid: { ...gridCurve.grid, columns: [0.1], values: gridCurve.grid.values.map(() => [0.5]) }
			}),
			'effects.1.curves.0.grid.columns'
		);
		fails(
			withCurve({
				...gridCurve,
				x2: { model: 'srgb', channel: 'green' },
				grid: {
					...gridCurve.grid,
					rows: Array.from({ length: 17 }, (_, index) => index / 16),
					values: Array.from({ length: 17 }, () => [0.5, 0.5])
				}
			}),
			'effects.1.curves.0.grid.rows'
		);
		fails(
			{
				effect: 'curves',
				enabled: true,
				channel: 'rgb',
				points: [
					[0, 0],
					[1, 1]
				]
			},
			'effects.1.channel'
		);
		fails({ effect: 'model-curves', enabled: true, model: 'hsl', curves: [] }, 'effects.1.effect');
		fails(
			{
				effect: 'channel-curve',
				enabled: true,
				x: { model: 'srgb', channel: 'red' },
				y: { model: 'srgb', channel: 'blue' },
				points: [
					[0, 0.5],
					[1, 0.5]
				]
			},
			'effects.1.effect'
		);
		const inherited = Object.setPrototypeOf(
			new Array(2),
			Object.assign(Object.create(Array.prototype), { 0: 0, 1: 0 })
		);
		fails(withCurve({ ...curve, points: [inherited, [1, 1]] }), 'effects.1.curves.0.points.0.0');
		fails({ ...grading['brightness-contrast'], contrast: 1.5 }, 'effects.1.contrast');
		fails({ ...grading.exposure, stops: -5 }, 'effects.1.stops');
		fails({ ...grading['white-balance'], temperature: Number.NaN }, 'effects.1.temperature');
		fails({ ...grading['hue-saturation'], hue: 200 }, 'effects.1.hue');
		fails({ ...grading['hue-saturation'], lightness: undefined }, 'effects.1.lightness');
	}));

test('every colour model accepts only its own channel names', () => {
	const byModel = {
		srgb: ['red', 'green', 'blue'],
		'linear-rgb': ['red', 'green', 'blue'],
		hsl: ['hue', 'saturation', 'lightness'],
		hsv: ['hue', 'saturation', 'value'],
		oklab: ['lightness', 'a', 'b'],
		oklch: ['lightness', 'chroma', 'hue'],
		cielab: ['lightness', 'a', 'b'],
		cielch: ['lightness', 'chroma', 'hue'],
		ycbcr: ['luma', 'cb', 'cr']
	};
	for (const [model, channels] of Object.entries(byModel)) {
		for (const channel of channels) {
			const periodic = channel === 'hue';
			assert.equal(
				isEffect({
					effect: 'curves',
					enabled: true,
					curves: [
						{
							kind: 'adjust',
							x: { model, channel },
							y: { model: 'srgb', channel: 'red' },
							points: periodic
								? [
										[0, 0.5],
										[0.5, 0.8],
										[1, 0.5]
									]
								: [
										[0, 0.25],
										[1, 0.75]
									]
						}
					]
				}),
				true,
				`${model}.${channel}`
			);
		}
	}
	assert.equal(
		isEffect({
			effect: 'curves',
			enabled: true,
			curves: [
				{
					kind: 'adjust',
					x: { model: 'hsv', channel: 'lightness' },
					y: { model: 'srgb', channel: 'red' },
					points: [
						[0, 0.5],
						[1, 0.5]
					]
				}
			]
		}),
		false
	);
});

const warm = [
	{ kind: 'color', rgb: [96, 0, 24] },
	{ kind: 'color', rgb: [237, 28, 36] },
	{ kind: 'color', rgb: [255, 127, 39] },
	{ kind: 'color', rgb: [249, 221, 59] }
];
const auto = (strength = 1) => ({ effect: 'recolour', enabled: true, strength, recipe: null });

test('analyzeRecolour returns an editable recipe that reproduces automatic recolouring', () =>
	withProcessor((processor) => {
		const context = { palette: warm, space: 'oklab' };
		const recipe = processor.analyzeRecolour({ version: 1, source: ramp(), effects: [], context });
		assert.equal(recipe.space, 'oklab');
		assert.ok(Array.isArray(recipe.tone) && recipe.tone.length >= 2);
		const automatic = apply(processor, [auto(0.7)], { context });
		const explicit = apply(processor, [{ ...auto(0.7), recipe }], { context });
		assert.deepEqual(explicit.data, automatic.data);
		assert.deepEqual(apply(processor, [auto(0)], { context }).data, ramp().data);
		const grey = apply(processor, [{ ...auto(), recipe: { ...recipe, chroma: 0, groups: [] } }], {
			context
		});
		for (let index = 0; index < grey.data.length; index += 4) {
			const [r, g, b] = grey.data.subarray(index, index + 3);
			assert.ok(Math.max(r, g, b) - Math.min(r, g, b) <= 1);
		}
		const cielab = processor.analyzeRecolour({
			version: 1,
			source: ramp(),
			effects: [],
			context: { ...context, space: 'cielab' }
		});
		assert.equal(cielab.space, 'cielab');
		assert.notDeepEqual(
			processor.analyzeRecolour({
				version: 1,
				source: ramp(),
				effects: [],
				context: { palette: palette, space: 'oklab' }
			}),
			recipe
		);
		// Analysis sees the image after earlier effects.
		const after = processor.analyzeRecolour({
			version: 1,
			source: ramp(),
			effects: [double],
			context
		});
		assert.deepEqual(
			apply(processor, [double, { ...auto(), recipe: after }], { context }).data,
			apply(processor, [double, auto()], { context }).data
		);
	}));

test('recolour composes into opaque process v2 with the request palette and match space', () =>
	withProcessor((processor) => {
		const recipe = { version: 2, effects: [halve, auto()], ...terminal, match: 'oklab-euclidean' };
		const composed = processor.process({ source: opaqueRamp(), palette: warm, recipe });
		const staged = processor.process({
			source: apply(processor, [halve, auto()], {
				source: opaqueRamp(),
				context: { palette: warm, space: 'oklab' }
			}),
			palette: warm,
			recipe: { version: 1, ...terminal, match: 'oklab-euclidean' }
		});
		assert.deepEqual(composed, staged);
	}));

test('recolour context and recipe errors name their fields', () =>
	withProcessor((processor) => {
		const recipe = processor.analyzeRecolour({
			version: 1,
			source: ramp(),
			effects: [],
			context: { palette: warm, space: 'oklab' }
		});
		const fails = (run, code, path) =>
			assert.throws(run, (error) => error.code === code && error.path === path, path);
		fails(() => apply(processor, [auto()]), 'invalid-request', 'context.palette');
		fails(
			() => apply(processor, [auto()], { context: { palette: warm } }),
			'invalid-request',
			'context.space'
		);
		fails(
			() => apply(processor, [{ ...auto(), recipe }], { context: { space: 'cielab' } }),
			'invalid-settings',
			'effects.0.recipe.space'
		);
		fails(
			() =>
				apply(processor, [{ ...auto(), recipe: { ...recipe, chroma: 3 } }], {
					context: { space: 'oklab' }
				}),
			'invalid-settings',
			'effects.0.recipe.chroma'
		);
		fails(
			() => apply(processor, [auto(2)], { context: { palette: warm, space: 'oklab' } }),
			'invalid-settings',
			'effects.0.strength'
		);
		fails(
			() =>
				processor.analyzeRecolour({
					version: 1,
					source: ramp(),
					effects: [],
					context: { space: 'oklab' }
				}),
			'invalid-request',
			'context.palette'
		);
		fails(
			() =>
				processor.process({
					source: ramp(),
					palette: warm,
					recipe: {
						version: 2,
						effects: [{ ...auto(), recipe }],
						...terminal,
						match: 'cielab-euclidean'
					}
				}),
			'invalid-settings',
			'recipe.effects.0.recipe.space'
		);
		fails(
			() =>
				processor.process({
					source: ramp(),
					palette: [{ kind: 'transparent' }],
					recipe: { version: 2, effects: [auto()], ...terminal }
				}),
			'invalid-request',
			'palette'
		);
	}));

test('isEffect vets one step without Wasm', () => {
	assert.equal(isEffect(halve), true);
	assert.equal(isEffect({ ...halve, enabled: false }), true);
	const boundaryCurve = {
		...grading.curves.curves[0],
		points: Array.from({ length: 16 }, (_, index) => [index / 15, index / 15])
	};
	assert.equal(isEffect({ ...grading.curves, curves: Array(16).fill(boundaryCurve) }), true);
	const columns = Array.from({ length: 48 }, (_, index) => index / 48);
	const rows = Array.from({ length: 16 }, (_, index) => index / 15);
	assert.equal(
		isEffect({
			...grading.curves,
			curves: [
				{
					kind: 'adjust',
					x: { model: 'hsl', channel: 'hue' },
					x2: { model: 'srgb', channel: 'green' },
					y: { model: 'oklch', channel: 'chroma' },
					grid: { columns, rows, values: rows.map(() => columns.map(() => 0.5)) }
				}
			]
		}),
		true
	);
	assert.equal(isEffect(neutral.curves), true);
	for (const step of [
		{ effect: 'exposure', enabled: true, stops: 100 },
		{ ...halve, channel: 'gray' },
		{ ...halve, input: { black: 0.6, white: 0.4 } },
		{
			effect: 'curves',
			enabled: true,
			curves: [
				{
					kind: 'remap',
					x: { model: 'srgb', channel: 'red' },
					y: { model: 'srgb', channel: 'red' },
					points: [
						[0.5, 0],
						[0.5, 1]
					]
				}
			]
		},
		{
			effect: 'curves',
			enabled: true,
			channel: 'rgb',
			points: [
				[0, 0],
				[1, 1]
			]
		},
		{ effect: 'model-curves', enabled: true, model: 'hsl', curves: [] },
		{
			effect: 'channel-curve',
			enabled: true,
			x: { model: 'srgb', channel: 'red' },
			y: { model: 'srgb', channel: 'blue' },
			points: [
				[0, 0.5],
				[1, 0.5]
			]
		},
		{ ...halve, extra: true },
		{ effect: 'blur', enabled: true },
		null
	])
		assert.equal(isEffect(step), false, JSON.stringify(step));
});

const lightness = { model: 'oklch', channel: 'lightness' };
/** A mask curve that is `value` everywhere. */
const constant = (value) => ({
	x: lightness,
	points: [
		[0, value],
		[1, value]
	]
});
/** Full strength where red is low, none where it is high: the ramp covers both. */
const lowRed = {
	x: { model: 'srgb', channel: 'red' },
	points: [
		[0, 1],
		[0.5, 0],
		[1, 0]
	]
};

test('masks hold a step back per colour, and full or empty masks change nothing', () =>
	withProcessor((processor) => {
		const source = ramp();
		const steps = [grading.exposure, grading.curves, grading['hue-saturation']];
		for (const step of steps) {
			const unmasked = apply(processor, [step]).data;
			assert.deepEqual(apply(processor, [{ ...step, mask: [] }]).data, unmasked);
			assert.deepEqual(apply(processor, [{ ...step, mask: [constant(1)] }]).data, unmasked);
			assert.deepEqual(apply(processor, [{ ...step, mask: [constant(0)] }]).data, source.data);
			const masked = apply(processor, [{ ...step, mask: [lowRed] }]).data;
			assert.notDeepEqual(masked, unmasked, step.effect);
			assert.notDeepEqual(masked, source.data, step.effect);
			for (let index = 3; index < source.data.length; index += 4)
				assert.equal(masked[index], source.data[index]);
		}
		const recolour = { effect: 'recolour', enabled: true, strength: 1, recipe: null };
		const context = { context: { palette, space: 'oklab' } };
		assert.deepEqual(
			apply(processor, [{ ...recolour, mask: [constant(0)] }], context).data,
			source.data
		);
		assert.ok(isEffect({ ...grading.exposure, mask: [lowRed, constant(0.5)] }));
	}));

test('masks validate their limits and curves with indexed paths', () =>
	withProcessor((processor) => {
		const fails = (mask, path) =>
			assert.throws(
				() => apply(processor, [halve, { ...grading.exposure, enabled: false, mask }]),
				(error) => error.code === 'invalid-settings' && error.path === path,
				path
			);
		fails(Array(5).fill(constant(1)), 'effects.1.mask');
		fails({}, 'effects.1.mask');
		fails(
			[constant(1), { ...constant(1), x: { model: 'hsl', channel: 'chroma' } }],
			'effects.1.mask.1.x.channel'
		);
		// Shape errors name the exact field, finer than Rust's `effects.1`.
		fails([{ ...constant(1), y: lightness }], 'effects.1.mask.0.y');
		fails([{ ...constant(1), kind: 'remap' }], 'effects.1.mask.0.kind');
		fails(
			[
				{
					x: lightness,
					points: [
						[0, 1],
						[1, 1.5]
					]
				}
			],
			'effects.1.mask.0.points.1.1'
		);
		fails(
			[
				{
					x: { model: 'hsv', channel: 'hue' },
					points: [
						[0, 1],
						[1, 0]
					]
				}
			],
			'effects.1.mask.0.points.1.1'
		);
		const grid = { x: twoInputCurve.x, x2: twoInputCurve.x2, grid: twoInputCurve.grid };
		fails([{ ...grid, x2: grid.x }], 'effects.1.mask.0.x2');
		fails(
			[
				{ ...grid, grid: { ...grid.grid, values: grid.grid.values.map((row) => row.map(() => 2)) } }
			],
			'effects.1.mask.0.grid.values.0.0'
		);
		// Arguments are checked before the mask, as in Rust.
		assert.throws(
			() => apply(processor, [{ ...grading.exposure, stops: 9, mask: Array(5).fill(constant(1)) }]),
			(error) => error.path === 'effects.0.stops'
		);
		assert.ok(apply(processor, [{ ...grading.exposure, mask: [grid] }]));
	}));

const fit = (overrides = {}) => ({
	effect: 'palette-fit',
	enabled: true,
	look: 'fitted',
	space: 'oklab',
	strength: 1,
	curves: null,
	...overrides
});
const fitContext = { palette };

test('analyzePaletteFit returns editable curves that reproduce automatic palette fit', () =>
	withProcessor((processor) => {
		const curves = processor.analyzePaletteFit({
			version: 1,
			source: ramp(),
			effects: [],
			look: 'fitted',
			space: 'oklab',
			context: fitContext
		});
		assert.ok(Array.isArray(curves) && curves.length > 0 && curves.length <= 5);
		const automatic = apply(processor, [fit({ strength: 0.7 })], { context: fitContext });
		assert.deepEqual(
			apply(processor, [fit({ strength: 0.7, curves })], { context: fitContext }).data,
			automatic.data,
			'explicit curves'
		);
		assert.deepEqual(
			apply(processor, [{ effect: 'curves', enabled: true, curves }], {
				context: fitContext
			}).data,
			apply(processor, [fit()], { context: fitContext }).data,
			'strength 1 equals a curves step'
		);
		assert.deepEqual(
			apply(processor, [fit({ strength: 0 })], { context: fitContext }).data,
			ramp().data,
			'strength 0 is a no-op'
		);
		// Analysis sees the image after earlier effects, and answers to palette and space.
		const after = processor.analyzePaletteFit({
			version: 1,
			source: ramp(),
			effects: [double],
			look: 'fitted',
			space: 'oklab',
			context: fitContext
		});
		assert.deepEqual(
			apply(processor, [double, fit({ curves: after })], { context: fitContext }).data,
			apply(processor, [double, fit()], { context: fitContext }).data
		);
		assert.notDeepEqual(
			processor.analyzePaletteFit({
				version: 1,
				source: ramp(),
				effects: [],
				look: 'fitted',
				space: 'cielab',
				context: fitContext
			}),
			curves
		);
		assert.notDeepEqual(
			processor.analyzePaletteFit({
				version: 1,
				source: ramp(),
				effects: [],
				look: 'fitted',
				space: 'oklab',
				context: { palette: warm }
			}),
			curves
		);
	}));

test('palette fit composes into process v2 and validates its fields', () =>
	withProcessor((processor) => {
		const recipe = { version: 2, effects: [fit()], ...terminal, match: 'oklab-euclidean' };
		const composed = processor.process({ source: opaqueRamp(), palette, recipe });
		const staged = processor.process({
			source: apply(processor, [fit()], { source: opaqueRamp(), context: fitContext }),
			palette,
			recipe: { version: 1, ...terminal, match: 'oklab-euclidean' }
		});
		assert.deepEqual(composed, staged);
		// An explicit list needs no palette at all.
		apply(processor, [fit({ curves: after_fit(processor) })], { context: {} });
		const fails = (run, code, path) =>
			assert.throws(run, (error) => error.code === code && error.path === path, path);
		fails(() => apply(processor, [fit()]), 'invalid-request', 'context.palette');
		fails(() => apply(processor, [fit({ strength: 2 })]), 'invalid-settings', 'effects.0.strength');
		fails(() => apply(processor, [fit({ look: 'natural' })]), 'invalid-settings', 'effects.0.look');
		fails(
			() => apply(processor, [fit({ space: 'hsv', curves: [] })]),
			'invalid-settings',
			'effects.0.space'
		);
		fails(
			() => apply(processor, [fit({ curves: [43] })]),
			'invalid-settings',
			'effects.0.curves.0'
		);
		fails(
			() =>
				processor.analyzePaletteFit({
					version: 1,
					source: ramp(),
					effects: [],
					look: 'fitted',
					space: 'oklab',
					context: { palette: [] }
				}),
			'invalid-request',
			'context.palette'
		);
		fails(
			() =>
				processor.analyzePaletteFit({
					version: 1,
					source: ramp(),
					effects: [],
					look: 'vivid',
					space: 'oklab',
					context: fitContext
				}),
			'invalid-settings',
			'look'
		);
	}));

function after_fit(processor) {
	return processor.analyzePaletteFit({
		version: 1,
		source: ramp(),
		effects: [],
		look: 'fitted',
		space: 'oklab',
		context: fitContext
	});
}
