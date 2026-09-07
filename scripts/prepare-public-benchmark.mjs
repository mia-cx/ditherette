#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, lstat, mkdir, readFile, readdir, realpath, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';
import { prepareTypeScript } from './prepare-benchmark-typescript.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const digest = (bytes) => [...createHash('sha256').update(bytes).digest()];
const sorted = (files) =>
	files.sort((a, b) => Buffer.compare(Buffer.from(a.path), Buffer.from(b.path)));
const command = (program, args, cwd = root) =>
	execFileSync(program, args, { cwd, encoding: 'utf8', maxBuffer: 16 * 1024 ** 2 }).trim();

/** Reject source changes before a build can create apparently revision-bound artifacts. */
export function cleanRevision(directory) {
	if (command('git', ['status', '--porcelain=v1', '--untracked-files=all'], directory)) {
		throw new Error('Public benchmark preparation requires a clean source checkout.');
	}
	return command('git', ['rev-parse', 'HEAD'], directory);
}

async function record(directory, name) {
	const bytes = await readFile(path.join(directory, name));
	return { path: name, bytes: bytes.length, digest: digest(bytes) };
}

/** Record every tracked source file, not a guessed set of compiler inputs. */
export async function sourceInventory(directory) {
	const names = execFileSync('git', ['ls-tree', '-rz', '--name-only', 'HEAD'], {
		cwd: directory,
		encoding: 'utf8'
	})
		.split('\0')
		.filter(Boolean);
	const files = [];
	for (const name of names) {
		if (!(await lstat(path.join(directory, name))).isFile()) {
			throw new Error(`Tracked input is not a regular file: ${name}`);
		}
		files.push(await record(directory, name));
	}
	return sorted(files);
}

/** Hash every file in a built tree; symlinks and special files are not build evidence. */
export async function fileInventory(directory, relative = '') {
	const files = [];
	for (const name of await readdir(path.join(directory, relative))) {
		const child = relative ? `${relative}/${name}` : name;
		const metadata = await lstat(path.join(directory, child));
		if (metadata.isDirectory()) files.push(...(await fileInventory(directory, child)));
		else if (metadata.isFile()) files.push(await record(directory, child));
		else throw new Error(`Built input is not a regular file: ${child}`);
	}
	return sorted(files);
}

async function tool(name, version, executable) {
	return { name, version, digest: digest(await readFile(executable)) };
}

async function buildTools() {
	const require = createRequire(new URL('../crates/ditherette-wasm/package.json', import.meta.url));
	const wasmPack = path.join(
		path.dirname(require.resolve('wasm-pack/package.json')),
		'binary/wasm-pack'
	);
	const pnpmEntry = await realpath(command('sh', ['-c', 'command -v pnpm']));
	const pnpmRoot = path.dirname(path.dirname(pnpmEntry));
	const tools = [
		await tool('node', process.version, process.execPath),
		await tool('typescript', ts.version, fileURLToPath(import.meta.resolve('typescript'))),
		await tool('wasm-pack', command(wasmPack, ['--version']), wasmPack),
		{
			name: 'pnpm-package',
			version: command('pnpm', ['--version']),
			digest: digest(JSON.stringify(await fileInventory(pnpmRoot)))
		}
	];
	for (const [variant, config] of [
		['scalar', 'rust-toolchain.toml'],
		['threads', 'crates/ditherette-wasm/rust-toolchain-threads.toml']
	]) {
		const channel = (await readFile(path.join(root, config), 'utf8')).match(
			/^channel = "([^"]+)"$/m
		)?.[1];
		if (!channel) throw new Error(`Missing compiler channel in ${config}`);
		for (const executable of ['rustc', 'cargo']) {
			const binary = command('rustup', ['which', '--toolchain', channel, executable]);
			tools.push(
				await tool(`${executable}-${variant}`, command(binary, ['--version', '--verbose']), binary)
			);
		}
	}
	// wasm-pack keeps downloaded executable tools outside the workspace. Record every cached version.
	const cache = path.join(
		process.env.XDG_CACHE_HOME || path.join(homedir(), '.cache'),
		'.wasm-pack'
	);
	for (const item of await fileInventory(cache)) {
		if (!['wasm-bindgen', 'wasm-opt'].includes(path.basename(item.path))) continue;
		const binary = path.join(cache, item.path);
		tools.push(await tool(`wasm-pack-cache/${item.path}`, command(binary, ['--version']), binary));
	}
	if (
		!tools.some(({ name }) => name.endsWith('/wasm-bindgen')) ||
		!tools.some(({ name }) => name.endsWith('/wasm-opt'))
	) {
		throw new Error('The actual wasm-pack compiler/optimizer cache was not found.');
	}
	return tools.sort((a, b) => a.name.localeCompare(b.name));
}

/** Build, pack, and install fresh artifacts before the benchmark lease or quiet phase. */
export async function preparePublicBenchmark(destination) {
	if (process.env.DITHERETTE_BENCH_QUIET === '1') {
		throw new Error('Build preparation must finish before the benchmark quiet phase.');
	}
	const revision = cleanRevision(root);
	const inputs = await sourceInventory(root);
	await mkdir(destination);
	const run = (program, args, cwd = root) => execFileSync(program, args, { cwd, stdio: 'inherit' });
	run('pnpm', ['--filter', 'ditherette', 'build']);
	const packageDirectory = path.join(root, 'packages/ditherette');
	const tarball = path.join(destination, 'ditherette.tgz');
	run('pnpm', ['pack', '--out', tarball], packageDirectory);
	const consumer = path.join(destination, 'consumer');
	await mkdir(consumer);
	await writeFile(
		path.join(consumer, 'package.json'),
		JSON.stringify({
			private: true,
			type: 'module',
			dependencies: { ditherette: `file:${tarball}` }
		})
	);
	run('pnpm', ['install', '--offline', '--ignore-scripts', '--lockfile=false'], consumer);
	const packagePath = await realpath(path.join(consumer, 'node_modules/ditherette'));
	const typescript = path.join(destination, 'typescript');
	await prepareTypeScript(typescript);
	const scripts = path.join(destination, 'scripts');
	await mkdir(scripts);
	for (const name of [
		'benchmark-public-browser.mjs',
		'benchmark-public-page.mjs',
		'benchmark-public-timing.mjs',
		'benchmark-transport.mjs'
	])
		await cp(path.join(root, 'scripts', name), path.join(scripts, name));
	const tools = await buildTools();
	if (
		cleanRevision(root) !== revision ||
		JSON.stringify(await sourceInventory(root)) !== JSON.stringify(inputs)
	) {
		throw new Error('Source inputs changed during public benchmark preparation.');
	}
	const provenance = path.join(destination, 'build-provenance.json');
	await writeFile(
		provenance,
		`${JSON.stringify(
			{
				schema: 1,
				source_revision: revision,
				tools,
				inputs,
				package: await fileInventory(packagePath),
				typescript: await fileInventory(typescript),
				scripts: await fileInventory(scripts)
			},
			null,
			2
		)}\n`
	);
	const bundle = {
		package: packagePath,
		typescript,
		scripts,
		source_checkout: root,
		provenance,
		entries: {
			package: 'package/dist/index.js',
			typescript: 'typescript/scripts/benchmark-typescript.js',
			transport: 'scripts/benchmark-public-browser.mjs',
			page: 'scripts/benchmark-public-page.mjs',
			wasm: 'package/dist/wasm/scalar/ditherette_wasm_bg.wasm'
		}
	};
	await writeFile(
		path.join(destination, 'bundle-source.json'),
		`${JSON.stringify(bundle, null, 2)}\n`
	);
	return bundle;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	if (process.argv.length !== 3)
		throw new Error('Usage: prepare-public-benchmark.mjs NEW_OUTPUT_DIRECTORY');
	console.log(JSON.stringify(await preparePublicBenchmark(path.resolve(process.argv[2]))));
}
