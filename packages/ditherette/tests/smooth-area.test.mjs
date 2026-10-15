import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette } from '../dist/index.js';
import { areaChecks, areaBudgetChecks } from './smooth-area-fixture.mjs';

test('complete 2D areas preserve Bayer and Sierra across sample sizes, tolerances and process routes', async (t) => {
	const wasm = await WebAssembly.compile(
		await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
	);
	const processor = await createDitherette({ wasm });
	try {
		assert.equal(areaChecks(processor).outputs.length, 54);
		const minima = await areaBudgetChecks((memoryLimitBytes) =>
			createDitherette({ wasm, memoryLimitBytes })
		);
		t.diagnostic(
			`Area minimum bytes (perturb, fused Bayer, resized diffusion): ${minima.join(', ')}`
		);
	} finally {
		processor.dispose();
	}
});
