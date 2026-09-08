import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette } from '../dist/index.js';
import { processBrowserChecks } from './process-browser-fixture.mjs';

const wasm = await WebAssembly.compile(
	await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
);
const request = () => ({
	source: { width: 2, height: 1, data: new Uint8Array([0, 0, 0, 0, 255, 255, 255, 255]) },
	palette: [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'transparent' }
	],
	recipe: {
		version: 1,
		output: { width: 4, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } },
		alpha: { mode: 'preserve', threshold: 128 },
		match: 'srgb-euclidean',
		dither: { family: 'none' }
	}
});

test('all filters and families compose with every matching mode and preserve request ownership', async () => {
	assert.deepEqual(
		await processBrowserChecks({
			wasm,
			moduleUrl: new URL('../dist/index.js', import.meta.url).href
		}),
		{
			compositions: 423,
			strictRequests: 9,
			caughtCopies: 3,
			exactBudget: true,
			scalarWithoutIsolation: true
		}
	);
});

test('public process accepts the versioned recipe and equals actual staged output', async () => {
	const processor = await createDitherette({ wasm });
	try {
		const input = request();
		const source = processor.resize({
			version: 1,
			source: input.source,
			output: input.recipe.output
		});
		const expected = processor.ditherAndQuantize({
			version: 1,
			source,
			palette: input.palette,
			alpha: input.recipe.alpha,
			matching: input.recipe.match,
			dither: input.recipe.dither
		});
		assert.deepEqual(processor.process(input), expected);
	} finally {
		processor.dispose();
	}
});
