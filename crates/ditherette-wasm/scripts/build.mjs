import { cp, readFile, rm } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const variant = process.argv[2];
if (variant !== 'scalar' && variant !== 'threads') {
	throw new Error('Expected scalar or threads.');
}

const crate = new URL('../', import.meta.url);
const root = new URL('../../', crate);
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
await cp(new URL('LICENSE', root), new URL(`dist/${variant}/LICENSE`, crate));
