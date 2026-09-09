import assert from 'node:assert/strict';

/** The 0.x version is the beta signal; release tags have no prerelease suffix. */
export function validateReleaseTag(version, tag) {
	assert.match(version, /^0\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)$/);
	assert.equal(tag, `v${version}`, 'Tag must exactly match the package and crate version.');
}

/** Validate the public distribution, not the generated private wasm-bindgen package manifests. */
export function validateManifest(manifest) {
	validateReleaseTag(manifest.version, `v${manifest.version}`);
	assert.equal(manifest.name, 'ditherette');
	assert.notEqual(manifest.private, true);
	assert.equal(manifest.type, 'module');
	assert.equal(manifest.license, 'MIT');
	assert.equal(manifest.sideEffects, false);
	assert.deepEqual(manifest.exports, {
		'.': { types: './dist/index.d.ts', import: './dist/index.js' }
	});
	assert.deepEqual(manifest.publishConfig, { access: 'public', tag: 'latest', provenance: true });
	assert.deepEqual(manifest.files, ['dist', 'README.md', 'LICENSE']);
	assert.equal(manifest.repository.url, 'https://github.com/mia-cx/ditherette.git');
	assert.equal(manifest.repository.directory, 'packages/ditherette');
	assert.equal(
		Object.keys(manifest.dependencies ?? {}).length,
		0,
		'Runtime assets are self-contained.'
	);
}

/** Reject stale/debug payloads while retaining the private JS helpers required by Wasm. */
export function validateFiles(files) {
	const names = files.map(({ path }) => path);
	assert.equal(new Set(names).size, names.length, 'Duplicate tarball paths.');
	for (const name of names) {
		assert.ok(!name.split('/').includes('..') && !name.startsWith('/'), name);
		assert.ok(
			['package.json', 'README.md', 'LICENSE'].includes(name) || name.startsWith('dist/'),
			name
		);
		assert.ok(
			!/(?:^|\/)(?:tests?|scripts?|target|bench[^/]*|build-mode\.json)(?:\/|$)/i.test(name),
			name
		);
		assert.ok(!/\.(?:rs|map|tsbuildinfo)$/.test(name), name);
		assert.ok(!name.endsWith('.ts') || name.endsWith('.d.ts'), name);
		if (name.startsWith('dist/'))
			assert.ok(
				/^dist\/[^/]+\.(?:js|d\.ts)$/.test(name) ||
					/^dist\/wasm\/(?:scalar|threads)\/(?:LICENSE|README\.md|package\.json|ditherette_wasm(?:\.factory)?\.(?:js|d\.ts)|ditherette_wasm_bg\.wasm(?:\.d\.ts)?)$/.test(
						name
					) ||
					/^dist\/wasm\/(?:scalar|threads)\/snippets\/[^/]+\/src\/(?:wasm\/)?[^/]+\.js$/.test(name),
				name
			);
	}
	for (const name of [
		'package.json',
		'README.md',
		'LICENSE',
		'dist/index.js',
		'dist/index.d.ts',
		'dist/thread-worker.js',
		'dist/worker-pool.js',
		'dist/threads.js'
	])
		assert.ok(names.includes(name), name);
	for (const variant of ['scalar', 'threads']) {
		for (const name of [
			'ditherette_wasm_bg.wasm',
			'ditherette_wasm.factory.js',
			'ditherette_wasm.factory.d.ts'
		])
			assert.ok(names.includes(`dist/wasm/${variant}/${name}`), `${variant}/${name}`);
	}
	assert.ok(
		names.some((name) =>
			/^dist\/wasm\/threads\/snippets\/[^/]+\/src\/workerHelpers\.no-bundler\.js$/.test(name)
		)
	);
}

/** Any new file or raw/compressed growth above 10% requires a reviewed budget update. */
export function sizeReview(sizes, policy) {
	assert.equal(policy.schema, 1);
	assert.equal(policy.maximumGrowth, 0.1);
	if (!policy.baseline) return ['Initial size baseline is not recorded.'];
	const findings = [];
	for (const item of [...sizes.files, { path: '$tarball', ...sizes.tarball }]) {
		const baseline = policy.baseline[item.path];
		if (!baseline) {
			findings.push(`${item.path}: new size item`);
			continue;
		}
		for (const metric of ['raw', 'gzip', 'brotli']) {
			if (item[metric] > baseline[metric] * (1 + policy.maximumGrowth))
				findings.push(`${item.path}: ${metric} grew more than ${policy.maximumGrowth * 100}%`);
		}
	}
	return findings;
}

export function requirePublication(policy, findings, environment, version, revision) {
	validateReleaseTag(version, environment.GITHUB_REF_NAME);
	assert.equal(environment.GITHUB_REF_TYPE, 'tag');
	assert.equal(environment.GITHUB_ACTIONS, 'true');
	assert.equal(environment.GITHUB_REPOSITORY, 'mia-cx/ditherette');
	assert.equal(environment.GITHUB_SHA, revision, 'Publish only the tested source revision.');
	assert.deepEqual(findings, [], 'Package size review is required.');
	assert.deepEqual(policy.holds, [], 'Recorded release holds must be resolved before publishing.');
}
