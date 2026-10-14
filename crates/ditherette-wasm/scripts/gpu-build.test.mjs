import assert from 'node:assert/strict';
import test from 'node:test';
import { buildConfiguration } from './build.mjs';

test('GPU is explicit and ordinary scalar/threaded configurations remain CPU-only', async () => {
	for (const variant of ['scalar', 'threads']) {
		const configuration = await buildConfiguration(variant);
		assert.ok(!configuration.args.includes('gpu'));
	}
	const gpu = await buildConfiguration('gpu');
	assert.deepEqual(gpu.args.slice(-2), ['--features', 'gpu']);
	assert.equal(gpu.threaded, false);
	assert.equal(gpu.toolchain, (await buildConfiguration('scalar')).toolchain);
});
