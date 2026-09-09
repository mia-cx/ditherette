import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
	requirePublication,
	sizeReview,
	validateFiles,
	validateManifest,
	validateReleaseTag
} from './release-contract.mjs';

const manifest = JSON.parse(await readFile(new URL('../package.json', import.meta.url)));
test('release identity is unscoped browser ESM and exact stable 0.x tags', () => {
	validateManifest(manifest);
	validateReleaseTag('0.1.0', 'v0.1.0');
	for (const [version, tag] of [
		['0.1.0', 'v0.1.1'],
		['0.1.0-beta.1', 'v0.1.0-beta.1'],
		['1.0.0', 'v1.0.0'],
		['0.01.0', 'v0.01.0'],
		['0.1.0', 'refs/tags/v0.1.0']
	])
		assert.throws(() => validateReleaseTag(version, tag));
	for (const change of [
		{ name: '@mia/ditherette' },
		{ private: true },
		{ sideEffects: true },
		{ exports: { ...manifest.exports, './raw': './dist/scalar.js' } },
		{ dependencies: { other: '1.0.0' } },
		{ publishConfig: { ...manifest.publishConfig, tag: 'beta' } }
	])
		assert.throws(() => validateManifest({ ...manifest, ...change }));
});

test('packed asset validation preserves both builds and excludes source/developer payload', () => {
	const files = [
		'package.json',
		'README.md',
		'LICENSE',
		'dist/index.js',
		'dist/index.d.ts',
		'dist/thread-worker.js',
		'dist/worker-pool.js',
		'dist/threads.js',
		'dist/wasm/threads/snippets/wasm-bindgen-rayon-hash/src/workerHelpers.no-bundler.js'
	];
	for (const variant of ['scalar', 'threads'])
		for (const name of [
			'ditherette_wasm_bg.wasm',
			'ditherette_wasm.factory.js',
			'ditherette_wasm.factory.d.ts'
		])
			files.push(`dist/wasm/${variant}/${name}`);
	const records = files.map((path) => ({ path }));
	validateFiles(records);
	for (const path of [
		'src/index.ts',
		'dist/source.rs',
		'dist/index.js.map',
		'dist/secret.json',
		'dist/bench-subjects.js',
		'dist/wasm/scalar/build-mode.json',
		'../escape',
		'/absolute'
	])
		assert.throws(() => validateFiles([...records, { path }]));
	assert.throws(() => validateFiles(records.filter(({ path }) => !path.includes('workerHelpers'))));
	assert.throws(() =>
		validateFiles(records.filter(({ path }) => !path.endsWith('threads/ditherette_wasm_bg.wasm')))
	);
});

test('ten-percent size boundary and unresolved publication holds fail closed', () => {
	const baseline = { raw: 100, gzip: 100, brotli: 100 };
	const policy = {
		schema: 1,
		maximumGrowth: 0.1,
		baseline: { file: baseline, $tarball: baseline },
		holds: []
	};
	const sizes = { files: [{ path: 'file', raw: 110, gzip: 110, brotli: 110 }], tarball: baseline };
	assert.deepEqual(sizeReview(sizes, policy), []);
	assert.equal(sizeReview({ ...sizes, tarball: { ...baseline, gzip: 111 } }, policy).length, 1);
	assert.equal(sizeReview({ ...sizes, files: [{ path: 'new', ...baseline }] }, policy).length, 1);
	assert.equal(sizeReview(sizes, { ...policy, baseline: null }).length, 1);
	const environment = {
		GITHUB_REF_NAME: 'v0.1.0',
		GITHUB_REF_TYPE: 'tag',
		GITHUB_ACTIONS: 'true',
		GITHUB_REPOSITORY: 'mia-cx/ditherette',
		GITHUB_SHA: 'source'
	};
	requirePublication(policy, [], environment, '0.1.0', 'source');
	assert.throws(() =>
		requirePublication(
			{ ...policy, holds: ['pending approval'] },
			[],
			environment,
			'0.1.0',
			'source'
		)
	);
	assert.throws(() => requirePublication(policy, ['size growth'], environment, '0.1.0', 'source'));
	for (const change of [
		{ GITHUB_REF_TYPE: 'branch' },
		{ GITHUB_REPOSITORY: 'other/repo' },
		{ GITHUB_ACTIONS: undefined },
		{ GITHUB_SHA: 'different' }
	])
		assert.throws(() =>
			requirePublication(policy, [], { ...environment, ...change }, '0.1.0', 'source')
		);
});

test('tag workflow validates the tested tarball before protected provenance publication', async () => {
	const workflow = await readFile(
		new URL('../../../.github/workflows/package-publish.yml', import.meta.url),
		'utf8'
	);
	for (const required of [
		'v0.*.*',
		'environment: npm-publish',
		'id-token: write',
		'shell: bash',
		'--frozen-lockfile',
		'release.mjs prepare',
		'test:conformance',
		'publish-check',
		'npm publish target/release/ditherette.tgz --access public --tag latest --provenance --ignore-scripts'
	])
		assert.ok(workflow.includes(required), required);
	assert.ok(workflow.indexOf('publish-check') < workflow.indexOf('npm publish target/release'));
	assert.ok(!/workflow_dispatch|NODE_AUTH_TOKEN|NPM_TOKEN|secrets\./.test(workflow));
});
