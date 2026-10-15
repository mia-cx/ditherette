import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette } from '../dist/index.js';
import { smoothGradientChecks, smoothGradientBudgetChecks } from './smooth-gradient-fixture.mjs';

test('versioned gradient placement controls real public fields, diffusion, process and validation', async (t) => {
	const wasm = await WebAssembly.compile(
		await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
	);
	const processor = await createDitherette({ wasm });
	try {
		const result = smoothGradientChecks(processor);
		assert.equal(result.outputs.length, 14);
		assert.ok(result.changedRepeated / result.repeated > 0.9);
		const minima = await smoothGradientBudgetChecks((memoryLimitBytes) =>
			createDitherette({ wasm, memoryLimitBytes })
		);
		assert.equal(minima.length, 3);
		t.diagnostic(
			`Scalar minimum bytes (perturb, fused Bayer, resized diffusion): ${minima.join(', ')}; repeated-byte pixels changed ${result.changedRepeated}/${result.repeated}.`
		);
	} finally {
		processor.dispose();
	}
});
