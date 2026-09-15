import { execFileSync } from 'node:child_process';
import {
	cpSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	rmSync,
	writeFileSync
} from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { CRATE, files } from './content.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
export const TOOLCHAIN = '1.97.0';
export const TARGETS = ['x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'];
const REFERENCE_DEPS = ['serde', 'serde_json', 'sha2'];

/** Cargo receives no caller-supplied flags, wrappers, target dirs, or toolchain override. */
export function cargoEnv() {
	return Object.fromEntries(
		Object.entries(process.env).filter(
			([key]) =>
				!/^(CARGO_|RUST|SCCACHE)/.test(key) || key === 'CARGO_HOME' || key === 'RUSTUP_HOME'
		)
	);
}

export function run(program, args, cwd, options = {}) {
	return execFileSync(program, args, {
		cwd,
		env: cargoEnv(),
		encoding: 'utf8',
		maxBuffer: 32 * 1024 * 1024,
		...options
	});
}

export function verifyCompiler() {
	const version = run('rustc', [`+${TOOLCHAIN}`, '-vV'], HERE);
	for (const line of [
		'release: 1.97.0',
		'commit-hash: 2d8144b7880597b6e6d3dfd63a9a9efae3f533d3',
		'LLVM version: 22.1.6'
	]) {
		if (!version.split('\n').includes(line))
			throw new Error(`Reference compiler mismatch: expected ${line}`);
	}
	return version.trim();
}

/** Audit before metadata: no candidate Cargo config or reference build-script execution. */
export function verifyBuildConfiguration(root) {
	const cargoHome = process.env.CARGO_HOME ?? join(homedir(), '.cargo');
	for (const path of [
		join(cargoHome, 'config'),
		join(cargoHome, 'config.toml'),
		join(tmpdir(), '.cargo/config'),
		join(tmpdir(), '.cargo/config.toml'),
		'/.cargo/config',
		'/.cargo/config.toml'
	]) {
		if (existsSync(path)) throw new Error(`Unaudited host Cargo configuration: ${path}`);
	}
	for (const directory of ['', 'crates', CRATE, 'crates/ditherette-bench']) {
		for (const name of ['.cargo/config', '.cargo/config.toml']) {
			if (existsSync(join(root, directory, name)))
				throw new Error(`Unaudited Cargo configuration: ${directory}/${name}`);
		}
	}
	if (existsSync(join(root, CRATE, 'build.rs')))
		throw new Error('Reference crate cannot acquire a build script');
	for (const path of ['Cargo.toml', 'crates/Cargo.toml']) {
		if (existsSync(join(root, path)))
			throw new Error(`Unaudited enclosing Cargo workspace: ${path}`);
	}
}

/** Cargo resolves aliases/features; only the reference's transitive registry closure is pinned. */
export function dependencyContext(root, manifest, target) {
	const metadata = JSON.parse(
		run(
			'cargo',
			[
				`+${TOOLCHAIN}`,
				'metadata',
				'--manifest-path',
				join(root, manifest),
				'--format-version',
				'1',
				'--locked',
				'--all-features',
				'--filter-platform',
				target
			],
			tmpdir()
		)
	);
	const packages = new Map(metadata.packages.map((p) => [p.id, p]));
	const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
	const core = metadata.packages.find((p) => p.name === 'ditherette-wasm');
	if (!core || core.edition !== '2021' || core.targets.some((t) => t.kind.includes('custom-build')))
		throw new Error('Changed reference crate edition or build script');
	const lib = core.targets.find((t) => t.kind.includes('rlib'));
	if (!lib || lib.src_path !== join(root, CRATE, 'src/lib.rs'))
		throw new Error('Changed reference crate root');
	const pending = REFERENCE_DEPS.map((name) => {
		const dependency = nodes.get(core.id).deps.find((d) => d.name === name);
		if (!dependency || packages.get(dependency.pkg).name !== name)
			throw new Error(`Reference dependency redirected: ${name}`);
		return dependency.pkg;
	});
	// Procedural macros can emit root items even from mutable adapters. Pin all
	// resolved macro implementations and their dependencies, not only their names.
	pending.push(
		...metadata.packages
			.filter((p) => p.targets.some((t) => t.kind.includes('proc-macro')) && nodes.has(p.id))
			.map((p) => p.id)
	);
	const closure = new Set();
	while (pending.length) {
		const id = pending.pop();
		if (closure.has(id)) continue;
		closure.add(id);
		pending.push(...nodes.get(id).dependencies);
	}
	// Cargo metadata omits registry checksums; use the matching lockfile package records.
	const lock = readFileSync(join(dirname(join(root, manifest)), 'Cargo.lock'), 'utf8');
	const checksums = new Map(
		lock
			.split('[[package]]')
			.slice(1)
			.map((part) => {
				const value = (key) => part.match(new RegExp(`^${key} = "([^"\\n]+)"`, 'm'))?.[1];
				return [`${value('name')}@${value('version')}@${value('source')}`, value('checksum')];
			})
	);
	return [...closure].sort().map((id) => {
		const { name, version, source } = packages.get(id);
		const checksum = checksums.get(`${name}@${version}@${source}`);
		if (source !== 'registry+https://github.com/rust-lang/crates.io-index' || !checksum)
			throw new Error(`Unpinned reference dependency: ${id}`);
		return {
			name,
			version,
			source,
			checksum,
			features: [...nodes.get(id).features].sort(),
			dependencies: [...nodes.get(id).dependencies].sort()
		};
	});
}

export function dependencySnapshot(root) {
	verifyBuildConfiguration(root);
	const result = {};
	for (const manifest of [`${CRATE}/Cargo.toml`, 'crates/ditherette-bench/Cargo.toml']) {
		for (const target of TARGETS)
			result[`${manifest}:${target}`] = dependencyContext(root, manifest, target);
	}
	return result;
}

export function verifyDependencies(root, expected) {
	const actual = dependencySnapshot(root);
	for (const [context, entries] of Object.entries(actual)) {
		if (JSON.stringify(entries) !== JSON.stringify(expected[context])) {
			const changed = entries.filter(
				(entry) =>
					JSON.stringify(entry) !==
					JSON.stringify(expected[context]?.find((e) => e.name === entry.name))
			);
			throw new Error(
				`Reference dependency/features changed in ${context}:\n${JSON.stringify(changed, null, 2)}`
			);
		}
	}
}

export function syntaxBinary() {
	run(
		'cargo',
		[`+${TOOLCHAIN}`, 'build', '--locked', '--manifest-path', join(HERE, 'syntax/Cargo.toml')],
		tmpdir(),
		{ stdio: ['ignore', 'inherit', 'inherit'] }
	);
	return join(HERE, 'syntax/target/debug/ditherette-freeze-syntax');
}

export function verifySyntax(root, binary = syntaxBinary()) {
	const args = [
		'root',
		join(root, CRATE, 'src/lib.rs'),
		'core-manifest',
		join(root, CRATE, 'Cargo.toml'),
		'bench-manifest',
		join(root, 'crates/ditherette-bench/Cargo.toml')
	];
	for (const role of ['spec', 'prod', 'image']) {
		for (const file of files(root, `${CRATE}/src/${role}`).filter((path) => path.endsWith('.rs')))
			args.push(role, join(root, file));
	}
	// Mutable adapters may consume both families, but cannot inject global macros
	// or redirect source loading around their frozen siblings.
	for (const file of files(root, `${CRATE}/src`).filter((path) => path.endsWith('.rs'))) {
		if (
			file === `${CRATE}/src/lib.rs` ||
			['spec', 'prod', 'image'].some((role) => file.startsWith(`${CRATE}/src/${role}/`))
		)
			continue;
		const role =
			file === `${CRATE}/src/wasm.rs` || file.startsWith(`${CRATE}/src/wasm/`) ? 'wasm' : 'adapter';
		args.push(role, join(root, file));
	}
	run(binary, args, tmpdir());
}

/** Copies only one semantic family. No opposite module or outside helper is available to Rust. */
export function isolatedCheck(root, role, target, threads = false) {
	const temporary = mkdtempSync(join(tmpdir(), 'ditherette-freeze-'));
	try {
		mkdirSync(join(temporary, 'src'));
		for (const folder of [role, 'image'])
			cpSync(join(root, CRATE, 'src', folder), join(temporary, 'src', folder), { recursive: true });
		writeFileSync(join(temporary, 'src/lib.rs'), `pub mod image;\npub mod ${role};\n`);
		writeFileSync(
			join(temporary, 'Cargo.toml'),
			`[package]\nname = "freeze-${role}"\nversion = "0.0.0"\nedition = "2021"\npublish = false\n[workspace]\n[features]\nthreads = ["dep:rayon"]\n[dependencies]\nserde = { version = "=1.0.228", features = ["derive"] }\nserde_json = "=1.0.149"\nsha2 = "=0.10.9"\nrayon = { version = "=1.12.0", optional = true }\n`
		);
		// Keep transitive versions from the validated core lock. Cargo removes unused entries.
		cpSync(join(root, CRATE, 'Cargo.lock'), join(temporary, 'Cargo.lock'));
		run(
			'cargo',
			[
				`+${TOOLCHAIN}`,
				'check',
				'--offline',
				'--manifest-path',
				join(temporary, 'Cargo.toml'),
				'--target',
				target,
				...(threads ? ['--features', 'threads'] : [])
			],
			tmpdir(),
			{
				env: { ...cargoEnv(), CARGO_TARGET_DIR: join(HERE, 'syntax/target/isolation') },
				stdio: ['ignore', 'inherit', 'inherit']
			}
		);
	} finally {
		rmSync(temporary, { recursive: true, force: true });
	}
}
