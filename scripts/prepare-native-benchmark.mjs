#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { chmod, copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
	assertBuildEnvironment,
	cleanRevision,
	sourceInventory
} from './prepare-public-benchmark.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const digest = (bytes) => [...createHash('sha256').update(bytes).digest()];
const runCommand = (program, args, cwd) =>
	execFileSync(program, args, {
		cwd,
		encoding: 'utf8',
		stdio: ['ignore', 'pipe', 'inherit'],
		maxBuffer: 16 * 1024 ** 2
	});

/** Invalidate only local release packages, preserving downloaded dependency caches. */
export async function buildFreshNative(directory, target, run = runCommand) {
	if (!path.isAbsolute(target) || path.parse(target).root === target) {
		throw new Error('Native preparation requires an explicit absolute target directory.');
	}
	const config = path.join(directory, 'rust-toolchain.toml');
	const channel = (await readFile(config, 'utf8')).match(/^channel = "([^"]+)"$/m)?.[1];
	if (!channel) throw new Error(`Missing compiler channel in ${config}`);
	const manifest = path.join(directory, 'crates/ditherette-bench/Cargo.toml');
	run(
		'cargo',
		[
			`+${channel}`,
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
		directory
	);
	run(
		'cargo',
		[
			`+${channel}`,
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
		directory
	);
}

/** Verify copied executable metadata against clean source and its complete observed bytes. */
export async function verifyNativeExecutable(executable, revision, run = runCommand) {
	const before = digest(await readFile(executable));
	const info = JSON.parse(run(executable, ['build-info'], path.dirname(executable)));
	if (
		info.build?.revision !== revision ||
		info.build?.dirty !== false ||
		typeof info.build?.rustc !== 'string' ||
		!info.build.rustc.startsWith('rustc ') ||
		typeof info.build?.tool_version !== 'string' ||
		!info.build.tool_version ||
		JSON.stringify(info.executable) !== JSON.stringify(before) ||
		JSON.stringify(digest(await readFile(executable))) !== JSON.stringify(before)
	) {
		throw new Error(
			'Native executable differs from its embedded clean revision or complete digest.'
		);
	}
	return info;
}

/** Prepare both binaries before quiet clearance; build-info never executes a workload. */
export async function prepareNativeBenchmark(
	destination,
	target,
	directory = root,
	run = runCommand
) {
	if (process.env.DITHERETTE_BENCH_QUIET === '1') {
		throw new Error('Build preparation must finish before the benchmark quiet phase.');
	}
	assertBuildEnvironment(process.env);
	const revision = cleanRevision(directory);
	const inputs = await sourceInventory(directory);
	await mkdir(destination);
	await buildFreshNative(directory, target, run);
	const executables = {};
	for (const name of ['ditherette-bench', 'ditherette-bench-pair']) {
		const executable = path.join(destination, name);
		await copyFile(path.join(target, 'release', name), executable);
		await chmod(executable, 0o555);
		executables[name] = {
			path: executable,
			...(await verifyNativeExecutable(executable, revision, run))
		};
	}
	if (
		cleanRevision(directory) !== revision ||
		JSON.stringify(await sourceInventory(directory)) !== JSON.stringify(inputs)
	) {
		throw new Error('Source inputs changed during native benchmark preparation.');
	}
	const provenance = { source_revision: revision, inputs, executables };
	await writeFile(
		path.join(destination, 'build-provenance.json'),
		`${JSON.stringify(provenance, null, 2)}\n`
	);
	return provenance;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	if (process.argv.length !== 4)
		throw new Error(
			'Usage: prepare-native-benchmark.mjs NEW_OUTPUT_DIRECTORY ABSOLUTE_TARGET_DIRECTORY'
		);
	const destination = path.resolve(process.argv[2]);
	const { source_revision, executables } = await prepareNativeBenchmark(
		destination,
		process.argv[3]
	);
	console.log(
		JSON.stringify({
			provenance: path.join(destination, 'build-provenance.json'),
			source_revision,
			executables
		})
	);
}
