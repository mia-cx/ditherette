import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isDeepStrictEqual } from 'node:util';
import { fileInventory } from '../../../scripts/prepare-public-benchmark.mjs';
import { frozenBrowserReference } from '../../../scripts/benchmark-public-browser.mjs';

/** Verify the actual frozen-only build inputs and bytes before serving an oracle context. */
export async function prepareYliluomaOracle(directory) {
	const oracle = process.env.DITHERETTE_BENCH_ORACLE;
	const fixtures = process.env.DITHERETTE_BENCH_YLILUOMA_FIXTURES;
	assert.ok(oracle, 'Set DITHERETTE_BENCH_ORACLE to a prepared frozen-only oracle.');
	assert.ok(fixtures, 'Set DITHERETTE_BENCH_YLILUOMA_FIXTURES to yliluoma_conformance output.');
	const manifest = JSON.parse(await readFile(join(oracle, 'manifest.json'), 'utf8'));
	const root = fileURLToPath(new URL('../../../', import.meta.url));
	const digest = (bytes) => [...createHash('sha256').update(bytes).digest()];
	assert.equal(
		manifest.frozen.artifact,
		'sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe'
	);
	assert.equal(manifest.target, 'wasm32-unknown-unknown');
	assert.deepEqual(manifest.profile, {
		release: true,
		opt_level: 's',
		wasm_opt: false,
		features: ['frozen-build']
	});
	for (const [base, files] of [
		[root, manifest.inputs],
		[oracle, manifest.files]
	]) {
		assert.ok(files.length);
		for (const file of files) {
			const bytes = await readFile(join(base, file.path));
			assert.equal(bytes.length, file.bytes, file.path);
			assert.deepEqual(digest(bytes), file.digest, file.path);
		}
	}
	const records = JSON.parse(await readFile(fixtures, 'utf8'));
	assert.equal(records.length, 367);
	const assetRoot = join(directory, 'oracle-assets');
	await mkdir(join(assetRoot, 'scripts'), { recursive: true });
	await cp(oracle, join(assetRoot, 'scripts/oracle'), { recursive: true });
	await cp(
		new URL('../../../scripts/benchmark-oracle-page.mjs', import.meta.url),
		join(assetRoot, 'scripts/benchmark-oracle-page.mjs')
	);
	await cp(
		new URL('../../../scripts/benchmark-indexed-wire.mjs', import.meta.url),
		join(assetRoot, 'scripts/benchmark-indexed-wire.mjs')
	);
	return {
		manifest,
		records,
		assets: { tree: { root: assetRoot, files: await fileInventory(assetRoot) }, entries: {} }
	};
}

/** Each permanent oracle context closes before any package context opens. No timers run. */
export async function yiluomaOracleChecks(browser, name, prepared, vectors, tarball) {
	const references = [];
	const differences = [];
	const cases = [];
	for (const [index, fixture] of prepared.records.entries()) {
		const raw = vectors.cases[index].request;
		assert.deepEqual(fixture.rgba, raw.source.data);
		assert.deepEqual(fixture.source, { width: raw.source.width, height: raw.source.height });
		assert.deepEqual(fixture.operation, {
			operation: 'yliluoma',
			settings: {
				quantize: { palette: raw.palette, alpha: raw.alpha, matching: raw.matching },
				size: raw.dither.size,
				placement: raw.dither.placement
			}
		});
		const reference = await frozenBrowserReference(browser, {
			browser: { assets: prepared.assets, runtime: { cross_origin_isolated: false } },
			case: {
				source: fixture.source,
				rgba: fixture.rgba,
				identity: fixture.identity,
				browser: { operation: fixture.operation },
				measurement: { samples: 1 }
			}
		});
		assert.equal(browser.contexts().length, 0, 'oracle context lifetime');
		assert.deepEqual(reference.case, fixture.identity);
		const { dimensions, pixels, warnings } = reference.output;
		const output = {
			...dimensions,
			indices: pixels.indices,
			palette: { rgba: pixels.palette_rgba, transparentIndex: pixels.transparent_index },
			warnings
		};
		assert.deepEqual(output, vectors.cases[index].output, `permanent frozen Wasm case ${index}`);
		if (!isDeepStrictEqual(reference.output, fixture.reference)) differences.push(index);
		references.push({ ...fixture, wasm_reference: reference });
		cases.push({ ...vectors.cases[index], output });
	}
	assert.deepEqual(differences, [236, 248, 251, 254, 257, 260, 263]);
	if (process.env.DITHERETTE_BENCH_ORACLE_EVIDENCE)
		await writeFile(
			join(process.env.DITHERETTE_BENCH_ORACLE_EVIDENCE, `${name}-yiluoma-references.json`),
			JSON.stringify({
				engine: name,
				version: browser.version(),
				oracle_manifest: prepared.manifest,
				tarball_sha256: createHash('sha256')
					.update(await readFile(tarball))
					.digest('hex'),
				differences,
				references
			}),
			{ flag: 'wx' }
		);
	return { ...vectors, cases };
}
