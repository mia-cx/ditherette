import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { pathToFileURL } from 'node:url';
import {
	buildConfiguration,
	writeBenchmarkMarker
} from '../crates/ditherette-wasm/scripts/build.mjs';
import {
	assertBuildEnvironment,
	buildFreshPackage,
	cleanRevision,
	fileInventory,
	sourceInventory,
	verifyPackageBuildMode
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
	calls.length = 0;
	await buildFreshPackage(directory, run, true);
	const packageDirectory = path.join(directory, 'packages/ditherette');
	assert.deepEqual(calls, [
		...expected.slice(0, 2),
		{ program: 'pnpm', args: ['check:version'], cwd: packageDirectory },
		...['scalar', 'threads'].map((variant) => ({
			program: 'pnpm',
			args: ['--filter', 'ditherette-wasm', `build:${variant}`, '--bench-subjects'],
			cwd: directory
		})),
		{ program: 'node', args: ['scripts/stage-wasm.mjs'], cwd: packageDirectory },
		{ program: 'pnpm', args: ['exec', 'tsc'], cwd: packageDirectory }
	]);

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

test('developer builds only add the explicit benchmark feature to normal compiler commands', async () => {
	for (const variant of ['scalar', 'threads']) {
		const normal = await buildConfiguration(variant);
		const developer = await buildConfiguration(variant, true);
		assert.deepEqual(developer, {
			...normal,
			args: [...normal.args, '--features', 'bench-subjects']
		});
		assert.ok(!normal.args.includes('bench-subjects'));
		assert.ok(normal.args.includes('--locked'));
		assert.ok(normal.rustFlags.includes('link-arg=--max-memory=2147483648'));
		assert.equal(normal.threaded, variant === 'threads');
	}
	await assert.rejects(buildConfiguration('unknown'), /Expected scalar or threads/);
});

test('both artifact variants require matching developer markers and raw Wasm exports', async (t) => {
	const directory = await mkdtemp(path.join(tmpdir(), 'ditherette-build-mode-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const header = [0, 97, 115, 109, 1, 0, 0, 0];
	const name = Buffer.from('privateExecutionPolicy');
	const developer = Buffer.from([
		...header,
		1,
		4,
		1,
		96,
		0,
		0, // One no-argument function type.
		3,
		2,
		1,
		0, // One function.
		7,
		name.length + 4,
		1,
		name.length,
		...name,
		0,
		0, // Its developer export.
		10,
		4,
		1,
		2,
		0,
		11 // An empty function body.
	]);
	for (const variant of ['scalar', 'threads']) {
		const output = path.join(directory, 'dist/wasm', variant);
		await mkdir(output, { recursive: true });
		await writeFile(path.join(output, 'ditherette_wasm_bg.wasm'), Buffer.from(header));
	}
	await verifyPackageBuildMode(directory, false);
	await assert.rejects(verifyPackageBuildMode(directory, true), /build mode/);
	for (const variant of ['scalar', 'threads']) {
		const output = path.join(directory, 'dist/wasm', variant);
		await writeFile(path.join(output, 'ditherette_wasm_bg.wasm'), developer);
	}
	await assert.rejects(verifyPackageBuildMode(directory, true), /benchmark build marker/);
	for (const variant of ['scalar', 'threads']) {
		const output = path.join(directory, 'dist/wasm', variant);
		await writeBenchmarkMarker(pathToFileURL(`${output}/`), variant);
		assert.deepEqual(JSON.parse(await readFile(path.join(output, 'build-mode.json'))), {
			schema: 1,
			mode: 'bench-subjects',
			variant
		});
	}
	await verifyPackageBuildMode(directory, true);
	await assert.rejects(verifyPackageBuildMode(directory, false), /build mode/);
	await writeFile(
		path.join(directory, 'dist/wasm/threads/build-mode.json'),
		'{"schema":1,"mode":"bench-subjects","variant":"scalar"}\n'
	);
	await assert.rejects(verifyPackageBuildMode(directory, true), /threads developer artifact/);
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
