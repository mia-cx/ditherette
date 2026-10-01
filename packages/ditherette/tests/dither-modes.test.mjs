import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette, DitheretteError } from '../dist/index.js';

const module = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const vectors = JSON.parse(
	await readFile(new URL('./fixtures/dither-modes.json', import.meta.url))
);
const source = { ...vectors.source, data: new Uint8Array(vectors.source.data) };
const errorIs = (code, path) => (error) =>
	error instanceof DitheretteError && error.code === code && error.path === path;

test('new kernels and ordered tiles match the dither_modes reference', async () => {
	const processor = await createDitherette({ wasm: module });
	try {
		for (const vector of vectors.diffusion) {
			const actual = processor.ditherAndQuantize({
				version: 1,
				source,
				palette: vectors.palette,
				alpha: vector.alpha,
				matching: vector.matching,
				dither: vector.dither
			});
			assert.deepEqual([...actual.indices], vector.indices, JSON.stringify(vector.dither));
		}
		for (const vector of vectors.ordered) {
			const label = JSON.stringify(vector.perturb);
			const rgba = processor.perturb({ version: 1, source, perturb: vector.perturb });
			assert.deepEqual([...rgba.data], vector.rgba, label);
			const indexed = processor.ditherAndQuantize({
				version: 1,
				source,
				palette: vectors.palette,
				alpha: vector.alpha,
				matching: vector.matching,
				dither: { family: 'separable', perturb: vector.perturb }
			});
			assert.deepEqual([...indexed.indices], vector.indices, label);
		}
		assert.equal(vectors.diffusion.length, 16);
		assert.equal(vectors.ordered.length, 8);
	} finally {
		processor.dispose();
	}
});

test('ordered tiles and new kernels reject malformed tags', async () => {
	const processor = await createDitherette({ wasm: module });
	const perturb = vectors.ordered[0].perturb;
	try {
		for (const [field, path] of [
			[{ algorithm: 'ordered', tile: '2x2' }, 'field.tile'],
			[{ algorithm: 'ordered' }, 'field.tile'],
			[{ algorithm: 'ordered', tile: '4x2', size: '4' }, 'field.size'],
			[{ algorithm: 'bayer', size: '4', tile: '4x2' }, 'field.tile'],
			[{ algorithm: 'blue-noise', tile: '4x2' }, 'field.tile']
		])
			assert.throws(
				() => processor.perturb({ version: 1, source, perturb: { ...perturb, field } }),
				errorIs('invalid-settings', `perturb.${path}`)
			);
		for (const kernel of ['sierra3', 'sierra-2-4a', 'dizzy', 'Stucki'])
			assert.throws(
				() =>
					processor.ditherAndQuantize({
						version: 1,
						source,
						palette: vectors.palette,
						alpha: vectors.diffusion[0].alpha,
						matching: 'srgb-euclidean',
						dither: { ...vectors.diffusion[0].dither, kernel }
					}),
				errorIs('invalid-settings', 'dither.kernel')
			);
	} finally {
		processor.dispose();
	}
});
