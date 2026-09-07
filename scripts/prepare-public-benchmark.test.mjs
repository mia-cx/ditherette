import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
	assertBuildEnvironment,
	buildFreshPackage,
	cleanRevision,
	fileInventory,
	sourceInventory
} from './prepare-public-benchmark.mjs';

test('each role rebuilds only the crate release Wasm outputs before package compilation', async (t) => {
	const directory = await mkdtemp(path.join(tmpdir(), 'ditherette-fresh-build-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const crate = path.join(directory, 'crates/ditherette-wasm');
	await mkdir(crate, { recursive: true });
	await writeFile(path.join(directory, 'rust-toolchain.toml'), 'channel = "1.97.0"\n');
	await writeFile(
		path.join(crate, 'rust-toolchain-threads.toml'),
		'channel = "nightly-2024-08-02"\n'
	);
	const calls = [];
	const run = (program, args, cwd) => calls.push({ program, args, cwd });
	const expected = ['scalar', 'threads'].map((variant, index) => ({
		program: 'cargo',
		args: [
			index === 0 ? '+1.97.0' : '+nightly-2024-08-02',
			'clean',
			'--package',
			'ditherette-wasm',
			'--release',
			'--target',
			'wasm32-unknown-unknown',
			'--target-dir',
			path.join(crate, 'target', variant)
		],
		cwd: crate
	}));
	expected.push({ program: 'pnpm', args: ['--filter', 'ditherette', 'build'], cwd: directory });
	await buildFreshPackage(directory, run);
	await buildFreshPackage(directory, run);
	assert.deepEqual(calls, [...expected, ...expected]);

	for (const failingVariant of ['scalar', 'threads']) {
		const attempted = [];
		await assert.rejects(
			buildFreshPackage(directory, (program, args) => {
				attempted.push(program);
				if (args.at(-1) === path.join(crate, 'target', failingVariant))
					throw new Error('clean failed');
			}),
			/clean failed/
		);
		assert.ok(attempted.every((program) => program === 'cargo'));
	}
});

test('source provenance rejects dirty input and records every tracked byte', async (t) => {
	const directory = await mkdtemp(path.join(tmpdir(), 'ditherette-source-provenance-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const git = (...args) => execFileSync('git', args, { cwd: directory, stdio: 'pipe' });
	git('init', '-q');
	await writeFile(path.join(directory, 'input'), 'first');
	git('add', 'input');
	git(
		'-c',
		'user.name=Fixture',
		'-c',
		'user.email=fixture@example.invalid',
		'commit',
		'-qm',
		'fixture'
	);
	assert.match(cleanRevision(directory), /^[0-9a-f]{40,64}$/);
	const before = await sourceInventory(directory);
	assert.equal(before.length, 1);
	assert.equal(before[0].path, 'input');
	assert.equal(before[0].bytes, 5);
	assert.equal(before[0].digest.length, 32);
	await writeFile(path.join(directory, 'input'), 'other');
	assert.throws(() => cleanRevision(directory), /clean source checkout/);
	await assert.rejects(sourceInventory(directory), /differs from HEAD/);
	for (const flag of ['--assume-unchanged', '--skip-worktree']) {
		git('update-index', flag, 'input');
		assert.match(cleanRevision(directory), /^[0-9a-f]{40,64}$/);
		await assert.rejects(sourceInventory(directory), /differs from HEAD/);
		git('update-index', flag.replace('--', '--no-'), 'input');
	}
});

test('compiler and profile overrides cannot claim the configured toolchain', () => {
	for (const name of [
		'RUSTC',
		'RUSTC_WRAPPER',
		'RUSTFLAGS',
		'CARGO_PROFILE_RELEASE_OPT_LEVEL',
		'NODE_OPTIONS'
	]) {
		assert.throws(() => assertBuildEnvironment({ [name]: 'override' }), /Unset build overrides/);
	}
	assert.doesNotThrow(() =>
		assertBuildEnvironment({ PATH: '/usr/bin', CARGO_HOME: '/cache', RUSTUP_HOME: '/rustup' })
	);
});

test('built provenance sorts full trees and rejects symbolic links', async (t) => {
	const directory = await mkdtemp(path.join(tmpdir(), 'ditherette-built-provenance-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	await mkdir(path.join(directory, 'nested'));
	await writeFile(path.join(directory, 'z'), 'z');
	await writeFile(path.join(directory, 'nested/a'), 'a');
	assert.deepEqual(
		(await fileInventory(directory)).map(({ path }) => path),
		['nested/a', 'z']
	);
	await symlink('z', path.join(directory, 'alias'));
	await assert.rejects(fileInventory(directory), /not a regular file/);
});
