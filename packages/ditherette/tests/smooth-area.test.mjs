import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { createDitherette } from '../dist/index.js';
import {
	areaChecks,
	areaBudgetChecks,
	featherChecks,
	adjacentToleranceChecks
} from './smooth-area-fixture.mjs';

test('increasing adjacent f32 tolerances preserves selected pixels', async () => {
	const wasm = await WebAssembly.compile(
		await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
	);
	const processor = await createDitherette({ wasm });
	try {
		const { selected } = adjacentToleranceChecks(processor);
		assert.equal(selected[1][3 * 76 + 52], true);
	} finally {
		processor.dispose();
	}
});

// Proves soft boundaries, gradient-side edge coverage, texture isolation and unchanged algorithm output.
test('regional boundaries feather density without losing the gradient side of lineart', async (t) => {
	const wasm = await WebAssembly.compile(
		await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
	);
	const processor = await createDitherette({ wasm });
	try {
		t.diagnostic(JSON.stringify(featherChecks(processor)));
	} finally {
		processor.dispose();
	}
});

// Proves spatial scale rejects compact transitions independently of their perceptual magnitude.
test('spatial scale distinguishes a compact transition from the same broad ramp', async (t) => {
	const wasm = await WebAssembly.compile(
		await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
	);
	const processor = await createDitherette({ wasm });
	try {
		for (const [name, length, high, largeSelected] of [
			['compact', 8, 192, false],
			['broad', 128, 192, true],
			['lower contrast compact', 8, 80, true]
		]) {
			const width = 512,
				height = 128,
				start = 192;
			const data = new Uint8Array(width * height * 4);
			for (let y = 0; y < height; y++)
				for (let x = 0; x < width; x++) {
					const value = Math.round(
						64 + (high - 64) * Math.max(0, Math.min(1, (x - start) / length))
					);
					data.set([value, value, value, 255], (y * width + x) * 4);
				}
			const source = { width, height, data };
			for (const sampleSize of [4, 64]) {
				const output = processor.perturb({
					version: 1,
					source,
					perturb: {
						field: { algorithm: 'bayer', size: '16' },
						space: 'srgb',
						strength: 8,
						placement: { mode: 'smooth-area-v1', sampleSize, contrastThreshold: 0.08 }
					}
				});
				let changed = 0;
				for (let y = 32; y < 96; y++)
					for (let x = start; x <= start + length; x++) {
						const p = (y * width + x) * 4;
						changed += Number(output.data[p] !== data[p]);
					}
				const total = (length + 1) * 64;
				t.diagnostic(
					`${name}, sample${sampleSize}: ${changed}/${total} transition pixels selected.`
				);
				assert.equal(
					changed,
					sampleSize === 4 || largeSelected ? total : 0,
					`${name}/sample${sampleSize}`
				);
			}
		}
	} finally {
		processor.dispose();
	}
});

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
