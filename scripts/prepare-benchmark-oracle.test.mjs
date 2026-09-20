import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
	buildFreshOracle,
	verifyOracleBuildConfiguration,
	verifyOracleManifests
} from './prepare-benchmark-oracle.mjs';

test(
	'oracle manifest policy rejects optimization, overflow, package overrides, patches, and build scripts',
	{ skip: !process.env.DITHERETTE_BENCH_PROFILE_CHECKER },
	async () => {
		const root = await mkdtemp(path.join(tmpdir(), 'oracle-profile-'));
		const manifest = path.join(root, 'crates/ditherette-bench-oracle/Cargo.toml');
		try {
			await mkdir(path.dirname(manifest), { recursive: true });
			await mkdir(path.join(root, 'crates/ditherette-bench-api'), { recursive: true });
			await writeFile(
				path.join(root, 'crates/ditherette-bench-api/Cargo.toml'),
				'[package]\nname="api"\nversion="0.1.0"\n'
			);
			const base = '[package]\nname="oracle"\nversion="0.1.0"\n[profile.release]\nopt-level="s"\n';
			await writeFile(manifest, base);
			verifyOracleManifests(root, process.env.DITHERETTE_BENCH_PROFILE_CHECKER);
			for (const text of [
				base.replace('"s"', '3'),
				`${base}overflow-checks=true\n`,
				`${base}lto=true\n`,
				`${base}panic="abort"\n`,
				`${base}[profile.release.package.foo]\nopt-level=3\n`,
				`${base}[patch.crates-io]\nfoo={path="../foo"}\n`,
				base.replace('[profile.release]', 'build="other.rs"\n[profile.release]')
			]) {
				await writeFile(manifest, text);
				assert.throws(() =>
					verifyOracleManifests(root, process.env.DITHERETTE_BENCH_PROFILE_CHECKER)
				);
			}
		} finally {
			await rm(root, { recursive: true });
		}
	}
);

test('oracle preparation rejects local compiler redirects and custom build scripts before metadata', async () => {
	const root = await mkdtemp(path.join(tmpdir(), 'oracle-build-config-'));
	try {
		for (const name of ['ditherette-bench-oracle', 'ditherette-bench-api']) {
			for (const relative of [
				'.cargo/config',
				'.cargo/config.toml',
				'build.rs',
				'rust-toolchain',
				'rust-toolchain.toml'
			]) {
				const file = path.join(root, 'crates', name, relative);
				await mkdir(path.dirname(file), { recursive: true });
				await writeFile(file, 'unaudited');
				assert.throws(
					() => verifyOracleBuildConfiguration(root),
					/Unaudited oracle build configuration/
				);
				await rm(file);
			}
		}
		verifyOracleBuildConfiguration(root);
	} finally {
		await rm(root, { recursive: true });
	}
});

test('fresh oracle preparation cleans both local release packages without clearing registry or role evidence', () => {
	const calls = [];
	buildFreshOracle((args) => calls.push(args), '/exclusive/scalar');
	assert.deepEqual(calls, [
		[
			'clean',
			'--package',
			'ditherette-bench-oracle',
			'--package',
			'ditherette-bench-api',
			'--release',
			'--target',
			'wasm32-unknown-unknown',
			'--target-dir',
			'/exclusive/scalar'
		],
		[
			'build',
			'--locked',
			'--offline',
			'--features',
			'frozen-build',
			'--release',
			'--target',
			'wasm32-unknown-unknown',
			'--target-dir',
			'/exclusive/scalar'
		]
	]);
});
