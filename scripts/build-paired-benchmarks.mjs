#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const self = fileURLToPath(import.meta.url);
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

export function normalizedArguments(args, environment) {
	if (args.some((arg) => arg.startsWith('@'))) {
		throw new Error('Recorded paired builds do not accept compiler response files');
	}
	const normalized = [];
	for (let i = 0; i < args.length; i += 1) {
		const arg = args[i];
		const codegen = arg === '-C' || arg === '--codegen' ? args[i + 1] :
			arg.startsWith('-C') ? arg.slice(2) :
			arg.startsWith('--codegen=') ? arg.slice('--codegen='.length) : '';
		if (codegen === 'target-cpu=native') {
			throw new Error('Recorded paired builds require an explicit CPU; target-cpu=native is host-dependent');
		}
		if (['--out-dir', '--error-format', '--json', '--color', '--diagnostic-width'].includes(arg)) {
			i += 1;
			continue;
		}
		if (arg === '-C' && /^(metadata|extra-filename)=/.test(args[i + 1] ?? '')) {
			i += 1;
			continue;
		}
		if (/^-C(metadata|extra-filename)=/.test(arg)) continue;
		if (arg === '--extern') {
			normalized.push(arg, args[++i].split('=')[0]);
			continue;
		}
		let value = arg;
		for (const [path, replacement] of [
			[environment.CARGO_MANIFEST_DIR, '<crate>'],
			[environment.DITHERETTE_PAIR_BUILD_DIRECTORY, '<build>']
		]) {
			if (path) value = value.replaceAll(path, replacement);
		}
		normalized.push(value);
	}
	return normalized;
}

function compiler() {
	const [rustc, ...args] = process.argv.slice(2);
	if (basename(rustc) !== 'rustc') throw new Error('Paired builds require the direct pinned rustc compiler');
	const environment = { ...process.env };
	const packageName = environment.CARGO_PKG_NAME;
	const isBinary = args[args.indexOf('--crate-type') + 1] === 'bin';
	const isNative = packageName === 'ditherette-bench' && isBinary &&
		args[args.indexOf('--crate-name') + 1] === 'ditherette_bench';
	const record = JSON.stringify({
		package: packageName,
		version: environment.CARGO_PKG_VERSION,
		arguments: normalizedArguments(args, environment)
	});
	const directory = environment.DITHERETTE_PAIR_RECORD_DIRECTORY;
	if (isNative) {
		const dependencies = readdirSync(directory).sort().map((name) => readFileSync(join(directory, name), 'utf8'));
		if (dependencies.length === 0) throw new Error('Paired compiler recipe is missing dependency builds');
		environment.DITHERETTE_BENCH_CONFIGURATION = JSON.stringify({
			schema: 'ditherette-rustc-recipe-v1',
			recorder: digest(readFileSync(self)),
			commands: [...dependencies, record].sort()
		});
		environment.DITHERETTE_BENCH_RECORDED_BUILD = 'true';
	}
	const result = spawnSync(rustc, args, { env: environment, stdio: 'inherit' });
	if (result.error) throw result.error;
	if (result.status === 0 && packageName && !(packageName === 'ditherette-bench' && isBinary)) {
		writeFileSync(join(directory, `${digest(record)}.json`), record);
	}
	process.exitCode = result.status ?? 1;
}

function build() {
	const [worktree, output, ...overrides] = process.argv.slice(2);
	if (!worktree || !output || overrides.length % 2 !== 0) {
		throw new Error('Usage: build-paired-benchmarks.mjs WORKTREE NEW_BUILD_DIRECTORY [--config profile.KEY=VALUE]');
	}
	for (let i = 0; i < overrides.length; i += 2) {
		if (overrides[i] !== '--config' || !/^profile\.[A-Za-z0-9_".*-]+\s*=\s*(true|false|[0-9]+|"[A-Za-z0-9_-]+")$/.test(overrides[i + 1])) {
			throw new Error('Only explicit Cargo profile overrides are supported');
		}
	}
	const directory = resolve(output);
	mkdirSync(directory);
	const records = join(directory, 'recipes');
	mkdirSync(records);
	const pinned = spawnSync('rustup', ['which', '--toolchain', '1.97.0', 'rustc'], { encoding: 'utf8' });
	if (pinned.error) throw pinned.error;
	if (pinned.status !== 0) throw new Error(pinned.stderr);
	const rustc = pinned.stdout.trim();
	const version = spawnSync(rustc, ['--version', '--verbose'], { encoding: 'utf8' });
	const host = version.stdout?.match(/^host: (.+)$/m)?.[1];
	if (version.status !== 0 || !host) throw new Error('Cannot determine pinned compiler host');
	const target = join(directory, 'target');
	const result = spawnSync('cargo', [
		'+1.97.0', 'build', '--locked', '--release', '--bins', '--target', host,
		'--manifest-path', join(resolve(worktree), 'crates/ditherette-bench/Cargo.toml'),
		'--target-dir', target, '--config', `build.build-dir=${JSON.stringify(target)}`,
		'--message-format=json-render-diagnostics', ...overrides
	], {
		cwd: resolve(worktree),
		env: {
			...process.env,
			RUSTC: rustc,
			RUSTC_WRAPPER: self,
			RUSTC_WORKSPACE_WRAPPER: '',
			CARGO_INCREMENTAL: '0',
			DITHERETTE_PAIR_RECORD_DIRECTORY: records,
			DITHERETTE_PAIR_BUILD_DIRECTORY: directory
		},
		encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], maxBuffer: 32 * 1024 * 1024
	});
	if (result.error) throw result.error;
	if (result.status !== 0) throw new Error(`Paired build failed (${result.status})`);
	const artifacts = result.stdout.split('\n').filter(Boolean).map((line) => JSON.parse(line))
		.filter((entry) => entry.reason === 'compiler-artifact' && entry.executable);
	if (!artifacts.some((entry) => entry.target.name === 'ditherette-bench')) {
		throw new Error('Cargo did not produce the paired benchmark executable');
	}
	for (const artifact of artifacts) process.stdout.write(`${artifact.target.name}\t${artifact.executable}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === self) {
	if (process.env.DITHERETTE_PAIR_RECORD_DIRECTORY) compiler();
	else build();
}
