import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette } from '../dist/index.js';

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

test('process v2 equals applyEffects then process v1, with an effects progress stage', () =>
	withProcessor((processor) => {
		const effects = [levels([0.1, 0.8], 1.3, [0, 1]), levels([0, 1], 1, [0.3, 0.6], 'blue')];
		const stages = [];
		const composed = processor.process({
			source: ramp(),
			palette,
			recipe: { version: 2, effects, ...terminal },
			onProgress: ({ stage }) => stages.push(stage)
		});
		const staged = processor.process({
			source: apply(processor, effects),
			palette,
			recipe: { version: 1, ...terminal }
		});
		assert.deepEqual(composed, staged);
		assert.equal(stages[0], 'prepare');
		assert.ok(stages.indexOf('effects') > 0, stages.join());
		assert.ok(stages.indexOf('effects') < stages.indexOf('resize'), stages.join());
		assert.equal(stages.at(-1), 'complete');
		const plain = processor.process({
			source: ramp(),
			palette,
			recipe: { version: 2, effects: [], ...terminal }
		});
		assert.deepEqual(
			plain,
			processor.process({ source: ramp(), palette, recipe: { version: 1, ...terminal } })
		);
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
