import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
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

test('ordinary Engine CI reaches the public fallback and explicitly builds the optional ABI', async () => {
	const root = new URL('../../../', import.meta.url);
	const manifest = JSON.parse(
		await readFile(new URL('packages/ditherette/package.json', root), 'utf8')
	);
	assert.ok(manifest.scripts['test:interface'].includes('tests/gpu-effects.test.mjs'));
	const command = manifest.scripts['test:gpu'];
	assert.match(command, /scripts\/build\.mjs gpu/);
	assert.match(command, /scripts\/stage-wasm\.mjs gpu/);
	for (const suite of [
		'gpu-build',
		'gpu-effects-bindings',
		'gpu-effects-admission',
		'gpu-effects-transport'
	])
		assert.ok(command.includes(`${suite}.test.mjs`));
	const workflow = await readFile(new URL('.github/workflows/ci.yml', root), 'utf8');
	const engine = workflow.slice(
		workflow.indexOf('\n  engine:'),
		workflow.indexOf('\n  release-tooling:')
	);
	assert.match(engine, /pnpm --filter ditherette test:interface/);
	assert.match(engine, /pnpm --filter ditherette test:gpu/);
});
