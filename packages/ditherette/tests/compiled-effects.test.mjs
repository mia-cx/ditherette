import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
	applyCompiledEffects,
	compileEffectMask,
	compileEffects,
	createDitherette,
	DitheretteError,
	indexColours
} from '../dist/index.js';

const wasm = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);

/** Gradients, a repeated flat, and every alpha class, so colours repeat across alphas. */
function photo() {
	const [width, height] = [40, 24];
	const data = new Uint8Array(width * height * 4);
	for (let y = 0; y < height; y++)
		for (let x = 0; x < width; x++) {
			const flat = x < 6;
			data.set(
				[
					flat ? 200 : (x * 37 + y * 11) & 255,
					flat ? 120 : (x * 5 + y * 23) & 255,
					flat ? 40 : (x * 13 + y * 7) & 255,
					y === 0 ? 0 : y === 1 ? 128 : 255
				],
				(y * width + x) * 4
			);
		}
	return { width, height, data };
}

const context = {
	palette: [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'color', rgb: [208, 48, 48] },
		{ kind: 'color', rgb: [48, 160, 80] },
		{ kind: 'color', rgb: [48, 80, 208] }
	],
	space: 'oklab'
};

const CHAINS = {
	tone: [
		{
			effect: 'levels',
			enabled: true,
			channel: 'rgb',
			input: { black: 0.1, white: 0.9 },
			gamma: 1.4,
			output: { black: 0.02, white: 0.98 }
		},
		{ effect: 'exposure', enabled: true, stops: 0.7 }
	],
	colour: [
		{ effect: 'hue-saturation', enabled: true, hue: 0.2, saturation: 0.4, lightness: -0.1 },
		{
			effect: 'curves',
			enabled: true,
			curves: [
				{
					kind: 'remap',
					x: { model: 'oklch', channel: 'lightness' },
					y: { model: 'oklch', channel: 'lightness' },
					points: [
						[0, 0],
						[0.5, 0.62],
						[1, 1]
					]
				}
			]
		}
	],
	'palette fit': [
		{ effect: 'exposure', enabled: true, stops: -0.3 },
		{ effect: 'recolour', enabled: true, strength: 0.8, recipe: null }
	],
	'curve fit': [
		{
			effect: 'palette-fit',
			enabled: true,
			look: 'fitted',
			space: 'oklab',
			strength: 0.9,
			curves: null
		}
	],
	// Masks read only the colour entering their step, so the chain stays exact per colour.
	masked: [
		{
			effect: 'exposure',
			enabled: true,
			stops: 0.8,
			mask: [
				{
					x: { model: 'oklch', channel: 'lightness' },
					points: [
						[0, 1],
						[0.6, 0.2],
						[1, 0]
					]
				}
			]
		},
		{
			effect: 'recolour',
			enabled: true,
			strength: 1,
			recipe: null,
			mask: [
				{
					x: { model: 'oklch', channel: 'hue' },
					points: [
						[0, 0.3],
						[0.4, 1],
						[1, 0.3]
					]
				}
			]
		}
	]
};

async function withProcessor(run) {
	const processor = await createDitherette({ wasm });
	try {
		await run(processor);
	} finally {
		processor.dispose();
	}
}

/** Give each analysis step the data `applyEffects` would derive from `image`. */
function resolved(processor, image, effects) {
	return effects.map((step, index) => {
		if (step.effect === 'recolour' && step.recipe === null)
			return {
				...step,
				recipe: processor.analyzeRecolour({
					version: 1,
					source: image,
					effects: effects.slice(0, index),
					context
				})
			};
		if (step.effect === 'palette-fit' && step.curves === null)
			return {
				...step,
				curves: processor.analyzePaletteFit({
					version: 1,
					source: image,
					effects: effects.slice(0, index),
					look: step.look,
					space: step.space,
					context
				})
			};
		return step;
	});
}

test('indexColours lists distinct colours in first-seen order, ignoring alpha', () => {
	const image = {
		width: 4,
		height: 1,
		data: new Uint8Array([1, 2, 3, 255, 4, 5, 6, 0, 1, 2, 3, 0, 4, 5, 6, 255])
	};
	const { colours, indices } = indexColours(image);
	assert.deepEqual([...colours], [0x030201, 0x060504]);
	assert.deepEqual([...indices], [0, 1, 0, 1]);
});

for (const [name, effects] of Object.entries(CHAINS))
	test(`compiled effects match applyEffects byte for byte: ${name}`, () =>
		withProcessor((processor) => {
			const image = photo();
			const { colours, indices } = indexColours(image);
			const results = compileEffects(processor, {
				version: 1,
				colours,
				effects: resolved(processor, image, effects),
				context
			});
			const expected = processor.applyEffects({ version: 1, source: image, effects, context });
			assert.deepEqual(applyCompiledEffects(image, indices, results).data, expected.data);
		}));

test('compileEffects asks for a palette fit recipe instead of analysing colours', () =>
	withProcessor((processor) => {
		const { colours } = indexColours(photo());
		assert.throws(
			() =>
				compileEffects(processor, {
					version: 1,
					colours,
					effects: CHAINS['palette fit'],
					context
				}),
			(error) => error instanceof DitheretteError && error.path === 'effects.1.recipe'
		);
		assert.throws(
			() =>
				compileEffects(processor, {
					version: 1,
					colours,
					effects: CHAINS['curve fit'],
					context
				}),
			(error) => error instanceof DitheretteError && error.path === 'effects.0.curves'
		);
	}));

test('applyCompiledEffects writes into a reused buffer of the right size', () => {
	const image = photo();
	const { colours, indices } = indexColours(image);
	const into = new Uint8Array(image.data.length);
	const mapped = applyCompiledEffects(image, indices, colours, into);
	assert.equal(mapped.data, into);
	// Identity results reproduce the image exactly, alpha included.
	assert.deepEqual(mapped.data, image.data);
});

test('compiled masks match effectMask byte for byte', () =>
	withProcessor((processor) => {
		const image = photo();
		const { colours, indices } = indexColours(image);
		const before = resolved(processor, image, CHAINS.masked.slice(0, 1));
		const { mask } = CHAINS.masked[1];
		const results = compileEffectMask(processor, {
			version: 1,
			colours,
			effects: before,
			mask,
			context
		});
		const expected = processor.effectMask({
			version: 1,
			source: image,
			effects: before,
			mask,
			context
		});
		assert.deepEqual(applyCompiledEffects(image, indices, results).data, expected.data);
		assert.ok(results.every((grey) => grey === (grey & 0xff) * 0x010101));
	}));

test('masks read the colour entering their step, not its clipped bytes', () =>
	withProcessor((processor) => {
		// Brightness 1 lifts this colour past white, but it stays orange, so a mask that holds
		// back every hue is 0 here, not the 1 a clipped white would read.
		const results = compileEffectMask(processor, {
			version: 1,
			colours: new Uint32Array([128 | (64 << 8) | (32 << 16)]),
			effects: [{ effect: 'brightness-contrast', enabled: true, brightness: 1, contrast: 0 }],
			mask: [
				{
					x: { model: 'oklch', channel: 'hue' },
					points: [
						[0, 0],
						[1, 0]
					]
				}
			]
		});
		assert.deepEqual([...results], [0]);
	}));

test('effectMask names a bad mask by its path', () =>
	withProcessor((processor) => {
		const curve = CHAINS.masked[0].mask[0];
		assert.throws(
			() =>
				processor.effectMask({
					version: 1,
					source: photo(),
					effects: [],
					mask: [curve, curve, curve, curve, curve]
				}),
			(error) =>
				error instanceof DitheretteError &&
				error.code === 'invalid-settings' &&
				error.path === 'mask'
		);
	}));
