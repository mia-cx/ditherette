import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile, realpath } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyContent } from '../tools/spec-freeze/content.mjs';
import { cargoEnv, verifyBuildConfiguration, verifyCompiler } from '../tools/spec-freeze/build.mjs';

export const FROZEN_DIGEST = '17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe';
const target = 'wasm32-unknown-unknown';
const digest = (bytes) => [...createHash('sha256').update(bytes).digest()];
const run = (program, args, cwd) =>
	execFileSync(program, args, {
		cwd,
		env: cargoEnv(),
		encoding: 'utf8',
		maxBuffer: 32 * 1024 ** 2
	}).trim();

/** The frozen guard audits the core crate; this standalone boundary needs its own local checks. */
export function verifyOracleBuildConfiguration(root) {
	verifyBuildConfiguration(root);
	for (const name of ['ditherette-bench-oracle', 'ditherette-bench-api']) {
		for (const file of [
			'.cargo/config',
			'.cargo/config.toml',
			'build.rs',
			'rust-toolchain',
			'rust-toolchain.toml'
		]) {
			if (existsSync(path.join(root, 'crates', name, file)))
				throw new Error(`Unaudited oracle build configuration: ${name}/${file}`);
		}
	}
}

/** Rebuild both local crates; registry caches and other package outputs remain intact. */
export function buildFreshOracle(build, cache) {
	build([
		'clean',
		'--package',
		'ditherette-bench-oracle',
		'--package',
		'ditherette-bench-api',
		'--release',
		'--target',
		target,
		'--target-dir',
		cache
	]);
	build([
		'build',
		'--locked',
		'--offline',
		'--features',
		'frozen-build',
		'--release',
		'--target',
		target,
		'--target-dir',
		cache
	]);
}

/** Reuse the frozen checker: exact release profile for the oracle, no profiles for its API dependency. */
export function verifyOracleManifests(root, checker) {
	run(
		checker,
		[
			'core-manifest',
			path.join(root, 'crates/ditherette-bench-oracle/Cargo.toml'),
			'bench-manifest',
			path.join(root, 'crates/ditherette-bench-api/Cargo.toml')
		],
		root
	);
}

async function sourceFiles(root, fileInventory) {
	const files = [];
	for (const prefix of [
		'crates/ditherette-bench-oracle',
		'crates/ditherette-bench-api',
		'crates/ditherette-wasm/src/spec',
		'crates/ditherette-wasm/src/image'
	])
		files.push(
			...(await fileInventory(path.join(root, prefix))).map((file) => ({
				...file,
				path: `${prefix}/${file.path}`
			}))
		);
	for (const name of [
		'crates/ditherette-bench/src/verification/identity.rs',
		'scripts/prepare-benchmark-oracle.mjs',
		'scripts/benchmark-oracle-page.mjs',
		'tools/spec-freeze/checkpoint.json',
		'tools/spec-freeze/dependencies.json',
		'tools/spec-freeze/content.mjs',
		'tools/spec-freeze/build.mjs'
	]) {
		const bytes = await readFile(path.join(root, name));
		files.push({ path: name, bytes: bytes.length, digest: digest(bytes) });
	}
	return files.sort((a, b) => Buffer.compare(Buffer.from(a.path), Buffer.from(b.path)));
}

/** Resolve the standalone crate against the already-approved frozen dependency closure. */
export async function oracleDependencies(root) {
	verifyOracleBuildConfiguration(root);
	const crate = path.join(root, 'crates/ditherette-bench-oracle');
	const metadata = JSON.parse(
		run(
			'cargo',
			[
				'+1.97.0',
				'metadata',
				'--features',
				'frozen-build',
				'--locked',
				'--offline',
				'--format-version',
				'1',
				'--filter-platform',
				target
			],
			crate
		)
	);
	const approved = JSON.parse(
		await readFile(path.join(root, 'tools/spec-freeze/dependencies.json'), 'utf8')
	)[`crates/ditherette-wasm/Cargo.toml:${target}`];
	const lock = await readFile(path.join(crate, 'Cargo.lock'), 'utf8');
	const checksums = new Map(
		lock
			.split('[[package]]')
			.slice(1)
			.map((part) => {
				const value = (key) => part.match(new RegExp(`^${key} = "([^"\\n]+)"`, 'm'))?.[1];
				return [`${value('name')}@${value('version')}`, value('checksum')];
			})
	);
	const dependencies = [];
	const coreMetadata = JSON.parse(
		run(
			'cargo',
			[
				'+1.97.0',
				'metadata',
				'--locked',
				'--offline',
				'--format-version',
				'1',
				'--filter-platform',
				target
			],
			path.join(root, 'crates/ditherette-wasm')
		)
	);
	const coreLock = await readFile(path.join(root, 'crates/ditherette-wasm/Cargo.lock'), 'utf8');
	for (const pkg of metadata.packages) {
		if (!pkg.source) {
			if (!['ditherette-bench-oracle', 'ditherette-bench-api'].includes(pkg.name))
				throw new Error('Oracle links an unauthorized local crate.');
			const expected = path.join(root, 'crates', pkg.name, 'src/lib.rs');
			if (
				pkg.targets.length !== 1 ||
				pkg.targets[0].src_path !== expected ||
				pkg.targets[0].kind.includes('custom-build')
			)
				throw new Error('Oracle local crate root or build target differs.');
			continue;
		}
		const node = metadata.resolve.nodes.find((node) => node.id === pkg.id);
		if (!node) continue;
		const record = {
			name: pkg.name,
			version: pkg.version,
			source: pkg.source,
			checksum: checksums.get(`${pkg.name}@${pkg.version}`),
			features: [...node.features].sort()
		};
		const corePackage = coreMetadata.packages.find((entry) => entry.name === pkg.name);
		const coreNode =
			corePackage && coreMetadata.resolve.nodes.find((entry) => entry.id === corePackage.id);
		const coreChecksum = coreLock
			.split('[[package]]')
			.find(
				(part) =>
					part.includes(`name = "${pkg.name}"\n`) && part.includes(`version = "${pkg.version}"\n`)
			)
			?.match(/^checksum = "([^"]+)"/m)?.[1];
		const expected =
			approved.find((entry) => entry.name === pkg.name) ??
			(corePackage &&
				coreNode && {
					version: corePackage.version,
					source: corePackage.source,
					checksum: coreChecksum,
					features: [...coreNode.features].sort()
				});
		if (
			!expected ||
			['version', 'source', 'checksum'].some((key) => record[key] !== expected[key]) ||
			JSON.stringify(record.features) !== JSON.stringify(expected.features)
		)
			throw new Error(
				`Oracle dependency differs from frozen build: ${JSON.stringify({ actual: record, expected })}`
			);
		dependencies.push(record);
	}
	return dependencies.sort((a, b) => a.name.localeCompare(b.name));
}

/** Fresh independent test-only Wasm build. Existing destinations and trial artifacts stay untouched. */
export async function prepareBenchmarkOracle(destination, root, fileInventory) {
	if (process.env.DITHERETTE_BENCH_QUIET === '1')
		throw new Error('Oracle builds are forbidden during a quiet phase.');
	verifyOracleBuildConfiguration(root);
	const compiler = verifyCompiler();
	const cache = path.join(root, 'crates/ditherette-wasm/target/scalar');
	run(
		'cargo',
		[
			'+1.97.0',
			'build',
			'--locked',
			'--offline',
			'--manifest-path',
			path.join(root, 'tools/spec-freeze/syntax/Cargo.toml'),
			'--target-dir',
			cache
		],
		root
	);
	const checker = path.join(cache, 'debug/ditherette-freeze-syntax');
	verifyOracleManifests(root, checker);
	const checkpoint = JSON.parse(
		await readFile(path.join(root, 'tools/spec-freeze/checkpoint.json'), 'utf8')
	);
	if (checkpoint.contentSha256 !== FROZEN_DIGEST)
		throw new Error('Oracle checkpoint differs from reviewed frozen identity.');
	const frozen = verifyContent(root, checkpoint);
	const dependencies = await oracleDependencies(root);
	const inputs = await sourceFiles(root, fileInventory);
	const crate = path.join(root, 'crates/ditherette-bench-oracle');
	const rustc = run('rustup', ['which', '--toolchain', '1.97.0', 'rustc'], root);
	const cargo = run('rustup', ['which', '--toolchain', '1.97.0', 'cargo'], root);
	const bindgen = path.join(
		process.env.XDG_CACHE_HOME ?? path.join(process.env.HOME, '.cache'),
		'.wasm-pack/wasm-bindgen-0abf6999c4bc0a86/wasm-bindgen'
	);
	if (run(bindgen, ['--version'], root) !== 'wasm-bindgen 0.2.121')
		throw new Error('Oracle binding compiler mismatch.');
	await mkdir(destination);
	const build = (args) =>
		execFileSync(cargo, args, {
			cwd: crate,
			env: { ...cargoEnv(), RUSTC: rustc },
			stdio: 'inherit'
		});
	buildFreshOracle(build, cache);
	run(
		bindgen,
		[
			path.join(cache, target, 'release/ditherette_bench_oracle.wasm'),
			'--target',
			'web',
			'--out-dir',
			destination,
			'--out-name',
			'ditherette_bench_oracle'
		],
		root
	);
	const tools = [];
	for (const [name, binary, version] of [
		['frozen-profile-checker', checker, 'trusted-core-manifest-v1'],
		['rustc', rustc, compiler],
		['cargo', cargo, run(cargo, ['--version', '--verbose'], root)],
		['wasm-bindgen', bindgen, '0.2.121']
	])
		tools.push({ name, version, digest: digest(await readFile(await realpath(binary))) });
	const sysroot = run(rustc, ['--print', 'sysroot'], root);
	const standardLibrary = await fileInventory(path.join(sysroot, 'lib/rustlib', target, 'lib'));
	const files = await fileInventory(destination);
	if (JSON.stringify(await sourceFiles(root, fileInventory)) !== JSON.stringify(inputs))
		throw new Error('Oracle source inputs changed during compilation.');
	const manifest = {
		schema: 1,
		frozen,
		target,
		profile: { release: true, opt_level: 's', wasm_opt: false, features: ['frozen-build'] },
		tools,
		standard_library: standardLibrary,
		dependencies,
		inputs,
		files
	};
	await writeFile(
		path.join(destination, 'manifest.json'),
		`${JSON.stringify(manifest, null, 2)}\n`,
		{ flag: 'wx' }
	);
	return manifest;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	if (process.argv.length !== 3)
		throw new Error('Usage: prepare-benchmark-oracle.mjs NEW_OUTPUT_DIRECTORY');
	const { fileInventory } = await import('./prepare-public-benchmark.mjs');
	await prepareBenchmarkOracle(
		path.resolve(process.argv[2]),
		path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'),
		fileInventory
	);
}
