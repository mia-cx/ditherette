import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { yiluomaBrowserChecks } from './yiluoma-browser-fixture.mjs';

test('public Yliluoma matches frozen vectors, strict controls, budgets and recoverable ownership', async () => {
	const vectors = JSON.parse(
		await readFile(new URL('./fixtures/yiluoma-wasm.json', import.meta.url))
	);
	const native = JSON.parse(await readFile(new URL('./fixtures/yiluoma.json', import.meta.url)));
	const targetDifferences = [];
	for (const [index, vector] of vectors.cases.entries()) {
		assert.deepEqual(vector.request, native.cases[index].request);
		if (JSON.stringify(vector.output) !== JSON.stringify(native.cases[index].output))
			targetDifferences.push(index);
	}
	assert.deepEqual(targetDifferences, [236, 248, 251, 254, 257, 260, 263]);
	const wasm = await WebAssembly.compile(
		await readFile(new URL('../dist/wasm/scalar/ditherette_wasm_bg.wasm', import.meta.url))
	);
	assert.deepEqual(await yiluomaBrowserChecks({ vectors, wasm }), {
		vectors: 367,
		caughtFailures: 3,
		strictControls: 8,
		exactBudget: true
	});
});
