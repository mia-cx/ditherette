import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {
	buildFreshNative,
	prepareNativeBenchmark,
	verifyNativeExecutable
} from './prepare-native-benchmark.mjs';

async function fixture(t) {
	const base = await mkdtemp(path.join(tmpdir(), 'ditherette-native-preparation-'));
	t.after(() => rm(base, { recursive: true, force: true }));
	const source = path.join(base, 'source');
	const target = path.join(base, 'target');
	await mkdir(source);
	await mkdir(path.join(target, 'release'), { recursive: true });
	await writeFile(path.join(source, 'rust-toolchain.toml'), 'channel = "1.97.0"\n');
	const git = (...args) =>
		execFileSync('git', args, { cwd: source, encoding: 'utf8', stdio: 'pipe' }).trim();
	git('init', '-q');
	git('add', '.');
	git(
		'-c',
		'user.name=Fixture',
		'-c',
		'user.email=fixture@example.invalid',
		'commit',
		'-qm',
		'fixture'
	);
	for (const name of ['ditherette-bench', 'ditherette-bench-pair'])
		await writeFile(path.join(target, 'release', name), `fake ${name}`);
	return { base, source, target, revision: git('rev-parse', 'HEAD') };
}

function metadata(executable, revision) {
	return {
		build: { revision, dirty: false, rustc: 'rustc fixture; host: fixture', tool_version: '0.1.0' },
		executable: [...createHash('sha256').update(readFileSync(executable)).digest()]
	};
}

test('native roles clean exactly three local release packages before the unchanged build recipe', async (t) => {
	const { source, target } = await fixture(t);
	const calls = [];
	const run = (program, args, cwd) => calls.push({ program, args, cwd });
	for (let role = 0; role < 2; role++) await buildFreshNative(source, target, run);
	const manifest = path.join(source, 'crates/ditherette-bench/Cargo.toml');
	const expected = [
		{
			program: 'cargo',
			args: [
				'+1.97.0',
				'clean',
				'--manifest-path',
				manifest,
				'--package',
				'ditherette-bench',
				'--package',
				'ditherette-bench-api',
				'--package',
				'ditherette-wasm',
				'--release',
				'--target-dir',
				target
			],
			cwd: source
		},
		{
			program: 'cargo',
			args: [
				'+1.97.0',
				'build',
				'--manifest-path',
				manifest,
				'--bins',
				'--examples',
				'--release',
				'--locked',
				'--target-dir',
				target
			],
			cwd: source
		}
	];
	assert.deepEqual(calls, [...expected, ...expected]);
	let count = 0;
	await assert.rejects(
		buildFreshNative(source, target, () => {
			count++;
			throw new Error('clean failed');
		}),
		/clean failed/
	);
	assert.equal(count, 1);
	for (const invalid of ['relative', '/'])
		await assert.rejects(buildFreshNative(source, invalid, run), /explicit absolute target/);
});

test('native preparation verifies copied binaries before writing handoff provenance', async (t) => {
	const { base, source, target, revision } = await fixture(t);
	const commands = [];
	const destination = path.join(base, 'prepared');
	const result = await prepareNativeBenchmark(destination, target, source, (program, args) => {
		commands.push({ program, args });
		if (program === 'cargo') return '';
		assert.deepEqual(args, ['build-info']);
		assert.equal(path.dirname(program), destination);
		return JSON.stringify(metadata(program, revision));
	});
	assert.equal(commands.length, 4);
	assert.equal(result.source_revision, revision);
	assert.equal(Object.keys(result.executables).length, 2);
	assert.deepEqual(
		JSON.parse(await readFile(path.join(destination, 'build-provenance.json'))),
		result
	);
	await assert.rejects(
		prepareNativeBenchmark(destination, target, source, () =>
			assert.fail('existing destination must not build')
		),
		/EEXIST/
	);

	const rejected = path.join(base, 'rejected');
	await assert.rejects(
		prepareNativeBenchmark(rejected, target, source, (program) =>
			program === 'cargo' ? '' : JSON.stringify(metadata(program, '0'.repeat(40)))
		),
		/embedded clean revision/
	);
	await assert.rejects(readFile(path.join(rejected, 'build-provenance.json')), /ENOENT/);
});

test('metadata rejects stale revision, dirty output, bad digest, malformed JSON and changing bytes', async (t) => {
	const { target, revision } = await fixture(t);
	const executable = path.join(target, 'release/ditherette-bench');
	for (const mutate of [
		(info) => {
			info.build.revision = '0'.repeat(40);
		},
		(info) => {
			info.build.dirty = true;
		},
		(info) => {
			info.executable[0] ^= 255;
		},
		(info) => {
			info.build.rustc = '';
		},
		(info) => {
			info.build.tool_version = '';
		}
	]) {
		await assert.rejects(
			verifyNativeExecutable(executable, revision, () => {
				const info = metadata(executable, revision);
				mutate(info);
				return JSON.stringify(info);
			}),
			/embedded clean revision/
		);
	}
	await assert.rejects(
		verifyNativeExecutable(executable, revision, () => 'not JSON'),
		SyntaxError
	);
	await assert.rejects(
		verifyNativeExecutable(executable, revision, () => {
			throw new Error('build-info failed');
		}),
		/build-info failed/
	);
	await assert.rejects(
		verifyNativeExecutable(executable, revision, () => {
			const info = metadata(executable, revision);
			writeFileSync(executable, 'changed during metadata probe');
			return JSON.stringify(info);
		}),
		/embedded clean revision/
	);
});
