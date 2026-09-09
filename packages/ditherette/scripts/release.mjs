import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { brotliCompressSync, constants, gunzipSync, gzipSync } from 'node:zlib';
import {
	assertBuildEnvironment,
	buildFreshPackage,
	cleanRevision,
	fileInventory,
	sourceInventory,
	verifyPackageBuildMode
} from '../../../scripts/prepare-public-benchmark.mjs';
import { verifyBuildConfiguration, verifyCompiler } from '../../../tools/spec-freeze/build.mjs';
import {
	requirePublication,
	sizeReview,
	validateFiles,
	validateManifest,
	validateReleaseTag
} from './release-contract.mjs';

const packageDirectory = fileURLToPath(new URL('../', import.meta.url));
const root = resolve(packageDirectory, '../..');
const command = (program, args, cwd = root) =>
	execFileSync(program, args, { cwd, encoding: 'utf8' }).trim();
const run = (program, args, cwd = root) => execFileSync(program, args, { cwd, stdio: 'inherit' });
const json = async (path) => JSON.parse(await readFile(path, 'utf8'));
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');
const serialize = (value) => `${JSON.stringify(value, null, 2)}\n`;

async function tools() {
	const workspace = await json(join(root, 'package.json'));
	const crate = await json(join(root, 'crates/ditherette-wasm/package.json'));
	assert.equal(workspace.packageManager, 'pnpm@11.13.0');
	assert.equal(workspace.engines.node, '24.19.0');
	assert.equal(crate.devDependencies['wasm-pack'], '0.15.0');
	for (const [file, channel] of [
		['rust-toolchain.toml', '1.97.0'],
		['crates/ditherette-wasm/rust-toolchain-threads.toml', 'nightly-2024-08-02']
	])
		assert.match(
			await readFile(join(root, file), 'utf8'),
			new RegExp(`^channel = "${channel}"$`, 'm')
		);
	const require = createRequire(join(root, 'crates/ditherette-wasm/package.json'));
	const wasmPack = join(dirname(require.resolve('wasm-pack/package.json')), 'binary/wasm-pack');
	const result = {
		node: process.version,
		pnpm: command('pnpm', ['--version']),
		npm: command('npm', ['--version']),
		wasmPack: command(wasmPack, ['--version']),
		scalarRustc: verifyCompiler(),
		threadedRustc: command('rustc', ['+nightly-2024-08-02', '--version', '--verbose'])
	};
	assert.equal(result.node, 'v24.19.0');
	assert.equal(result.pnpm, '11.13.0');
	assert.equal(result.npm, '11.17.0');
	assert.equal(result.wasmPack, 'wasm-pack 0.15.0');
	return result;
}

function compressed(bytes) {
	return {
		raw: bytes.length,
		gzip: gzipSync(bytes, { level: 9 }).length,
		brotli: brotliCompressSync(bytes, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length
	};
}

/** Measure the exact installed bytes and the complete uncompressed/compressed tar archive. */
export async function measure(packagePath, tarball) {
	const inventory = await fileInventory(packagePath);
	validateFiles(inventory);
	const packed = command('tar', ['-tzf', tarball]).split('\n');
	assert.deepEqual(
		packed.sort(),
		inventory.map(({ path }) => `package/${path}`).sort(),
		'Packed files match the installed inventory.'
	);
	validateManifest(await json(join(packagePath, 'package.json')));
	await verifyPackageBuildMode(packagePath, false);
	const files = [];
	for (const item of inventory) {
		const bytes = await readFile(join(packagePath, item.path));
		assert.deepEqual(
			execFileSync('tar', ['-xOf', tarball, `package/${item.path}`], { maxBuffer: 16 * 1024 ** 2 }),
			bytes,
			item.path
		);
		files.push({ path: item.path, sha256: sha256(bytes), ...compressed(bytes) });
	}
	const archive = await readFile(tarball);
	return {
		files,
		tarball: { ...compressed(gunzipSync(archive)), gzip: archive.length, sha256: sha256(archive) }
	};
}

/** Fresh ordinary compilation stays crate-owned; release evidence stays outside the tarball. */
export async function prepare(destination) {
	assertBuildEnvironment(process.env);
	assert.notEqual(
		process.env.DITHERETTE_BENCH_QUIET,
		'1',
		'Release builds require implementation phase.'
	);
	verifyBuildConfiguration(root);
	const revision = cleanRevision(root);
	const inputs = await sourceInventory(root);
	const versions = await tools();
	const manifest = await json(join(packageDirectory, 'package.json'));
	validateManifest(manifest);
	run('node', ['scripts/check-version.mjs'], packageDirectory);
	await mkdir(destination);
	await rm(join(packageDirectory, 'dist'), { recursive: true, force: true });
	await buildFreshPackage(root);
	await verifyPackageBuildMode(packageDirectory, false);
	const tarball = join(destination, 'ditherette.tgz');
	run('pnpm', ['pack', '--out', tarball], packageDirectory);
	const consumer = join(destination, 'consumer');
	await mkdir(consumer);
	await writeFile(
		join(consumer, 'package.json'),
		serialize({ private: true, type: 'module', dependencies: { ditherette: `file:${tarball}` } })
	);
	run(
		'pnpm',
		['install', '--offline', '--ignore-scripts', '--ignore-workspace', '--lockfile=false'],
		consumer
	);
	const packagePath = await realpath(join(consumer, 'node_modules/ditherette'));
	const sizes = await measure(packagePath, tarball);
	assert.equal(cleanRevision(root), revision);
	assert.deepEqual(
		await sourceInventory(root),
		inputs,
		'Source changed during release preparation.'
	);
	const report = {
		schema: 1,
		version: manifest.version,
		sourceRevision: revision,
		sourceCheckout: root,
		sourceDigest: sha256(JSON.stringify(inputs)),
		tools: versions,
		build_mode: 'public',
		compression: { gzipLevel: 9, brotliQuality: 11 },
		...sizes
	};
	await writeFile(join(destination, 'release-report.json'), serialize(report));
	await writeFile(
		join(destination, 'conformance-bundle.json'),
		serialize({
			package: packagePath,
			scripts: join(root, 'scripts'),
			provenance: join(destination, 'release-report.json')
		})
	);
	console.log(`Prepared ${tarball}; SHA-256 ${sizes.tarball.sha256}`);
	return report;
}

/** Offline verification binds the tarball, installed bytes, version, pins, and clean revision. */
export async function verify(destination, tag) {
	assertBuildEnvironment(process.env);
	verifyBuildConfiguration(root);
	const report = await json(join(destination, 'release-report.json'));
	assert.equal(report.schema, 1);
	assert.equal(report.build_mode, 'public');
	assert.equal(report.sourceRevision, cleanRevision(root));
	assert.equal(report.sourceDigest, sha256(JSON.stringify(await sourceInventory(root))));
	assert.deepEqual(report.tools, await tools());
	const manifest = await json(join(packageDirectory, 'package.json'));
	validateManifest(manifest);
	assert.equal(report.version, manifest.version);
	if (tag) validateReleaseTag(report.version, tag);
	run('node', ['scripts/check-version.mjs'], packageDirectory);
	const sizes = await measure(
		join(destination, 'consumer/node_modules/ditherette'),
		join(destination, 'ditherette.tgz')
	);
	assert.deepEqual(sizes, { files: report.files, tarball: report.tarball });
	const policy = await json(join(packageDirectory, 'release-policy.json'));
	const findings = sizeReview(sizes, policy);
	console.log(
		serialize({ tarballSha256: report.tarball.sha256, sizeReview: findings, holds: policy.holds })
	);
	return { report, policy, findings };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const [operation, directory, tag, ...extra] = process.argv.slice(2);
	assert.ok(
		directory && !extra.length,
		'Usage: release.mjs prepare|verify|publish-check DIRECTORY [TAG]'
	);
	const destination = resolve(directory);
	if (operation === 'prepare') {
		assert.equal(tag, undefined);
		await prepare(destination);
	} else if (operation === 'verify' || operation === 'publish-check') {
		const { report, policy, findings } = await verify(destination, tag);
		if (operation === 'publish-check')
			requirePublication(policy, findings, process.env, report.version, report.sourceRevision);
	} else throw new Error('Expected prepare, verify, or publish-check.');
}
