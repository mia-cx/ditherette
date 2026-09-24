import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { validateReleaseTag } from '../packages/ditherette/scripts/release-contract.mjs';

/** Update only the public crate entry; unrelated lockfile versions remain untouched. */
export function replaceCrateVersion(source, version, lockfile = false) {
	validateReleaseTag(version, `v${version}`);
	const pattern = lockfile
		? /(\[\[package\]\]\nname = "ditherette-wasm"\nversion = ")[^"]+("\n)/g
		: /(\[package\]\nname = "ditherette-wasm"\nversion = ")[^"]+("\n)/g;
	assert.equal([...source.matchAll(pattern)].length, 1, 'Expected exactly one public crate entry.');
	return source.replace(pattern, `$1${version}$2`);
}

export async function synchronizeCrateVersion(root) {
	const { version } = JSON.parse(await readFile(resolve(root, 'packages/ditherette/package.json')));
	for (const [path, lockfile] of [
		['crates/ditherette-wasm/Cargo.toml', false],
		['crates/ditherette-wasm/Cargo.lock', true],
		['crates/ditherette-bench/Cargo.lock', true]
	]) {
		const file = resolve(root, path);
		const before = await readFile(file, 'utf8');
		const after = replaceCrateVersion(before, version, lockfile);
		if (after !== before) await writeFile(file, after);
	}
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	await synchronizeCrateVersion(process.cwd());
}
