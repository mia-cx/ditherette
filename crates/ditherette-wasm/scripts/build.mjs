import { cp, readFile, rm, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { writeScalarFactory } from './scalar-factory.mjs';

const crate = new URL('../', import.meta.url);
const root = new URL('../../', crate);

/** Keep compiler, profile, linker flags, and staging identical for developer candidates. */
export async function buildConfiguration(variant, benchSubjects = false) {
	if (variant !== 'scalar' && variant !== 'threads') {
		throw new Error('Expected scalar or threads.');
	}
	const threaded = variant === 'threads';
	const toolchainFile = threaded
		? new URL('rust-toolchain-threads.toml', crate)
		: new URL('rust-toolchain.toml', root);
	const toolchain = (await readFile(toolchainFile, 'utf8')).match(/^channel = "([^"]+)"$/m)[1];
	const maxMemoryBytes = 2 * 1024 ** 3;
	const rustFlags = ['-C', `link-arg=--max-memory=${maxMemoryBytes}`];
	if (threaded) rustFlags.push('-C', 'target-feature=+atomics,+bulk-memory');

	const args = ['build', '.', '--target', 'web', '--out-dir', `dist/${variant}`, '--', '--locked'];
	if (threaded) args.push('--features', 'threads', '-Z', 'build-std=panic_abort,std');
	if (benchSubjects) args.push('--features', 'bench-subjects');
	return { args, toolchain, rustFlags, threaded };
}

/** The staged artifact inventory binds this developer-only marker to its Wasm bytes. */
export async function writeBenchmarkMarker(directory, variant) {
	await writeFile(
		new URL('build-mode.json', directory),
		`${JSON.stringify({ schema: 1, mode: 'bench-subjects', variant })}\n`
	);
}

async function build(variant, benchSubjects) {
	const { args, toolchain, rustFlags, threaded } = await buildConfiguration(variant, benchSubjects);
	await rm(new URL(`dist/${variant}/`, crate), { recursive: true, force: true });
	const result = spawnSync('wasm-pack', args, {
		cwd: crate,
		stdio: 'inherit',
		env: {
			...process.env,
			RUSTUP_TOOLCHAIN: toolchain,
			CARGO_ENCODED_RUSTFLAGS: rustFlags.join('\x1f'),
			CARGO_TARGET_DIR: fileURLToPath(new URL(`target/${variant}`, crate))
		}
	});
	if (result.error) throw result.error;
	if (result.status !== 0) process.exit(result.status ?? 1);
	await writeScalarFactory(new URL(`dist/${variant}/`, crate), threaded);
	await cp(new URL('LICENSE', root), new URL(`dist/${variant}/LICENSE`, crate));
	if (benchSubjects) await writeBenchmarkMarker(new URL(`dist/${variant}/`, crate), variant);
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
	const [variant, ...options] = process.argv.slice(2);
	if (options.length > 1 || (options.length === 1 && options[0] !== '--bench-subjects')) {
		throw new Error('Usage: build.mjs scalar|threads [--bench-subjects]');
	}
	await build(variant, options.length === 1);
}
