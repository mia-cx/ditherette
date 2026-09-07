import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { lstatSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

export const CRATE = 'crates/ditherette-wasm';
export const FROZEN_ROOTS = [`${CRATE}/src/spec`, `${CRATE}/src/image`];
export const GENERATOR = `${CRATE}/examples/generate_blue_noise.rs`;
export const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

/** Include untracked additions and file modes; never follow symlinks. */
export function files(root, relative) {
	const path = join(root, relative);
	const stat = lstatSync(path);
	if (stat.isSymbolicLink()) throw new Error(`Symlink is forbidden: ${relative}`);
	if (stat.isFile()) return [relative];
	if (!stat.isDirectory()) throw new Error(`Unsupported file type: ${relative}`);
	return readdirSync(path).sort().flatMap((name) => files(root, `${relative}/${name}`));
}

export function inventory(root, paths) {
	return paths.flatMap((path) => files(root, path)).sort().map((path) => ({
		path,
		mode: lstatSync(join(root, path)).mode & 0o111 ? '100755' : '100644',
		sha256: sha256(readFileSync(join(root, path)))
	}));
}

export function contentDigest(entries) {
	return sha256(entries.map(({ path, mode, sha256 }) => `${mode} ${sha256} ${path}\n`).join(''));
}

/** Compare against recorded bytes, never a merge base or the current parent. */
export function verifyContent(root, checkpoint) {
	const actual = inventory(root, [...FROZEN_ROOTS, GENERATOR]);
	if (JSON.stringify(actual) !== JSON.stringify(checkpoint.files)) {
		const expected = new Map(checkpoint.files.map((entry) => [entry.path, entry]));
		const found = new Map(actual.map((entry) => [entry.path, entry]));
		const changed = [...new Set([...expected.keys(), ...found.keys()])].filter(
			(path) => JSON.stringify(expected.get(path)) !== JSON.stringify(found.get(path))
		);
		throw new Error(`Frozen content changed:\n${changed.join('\n')}`);
	}
	if (contentDigest(actual) !== checkpoint.contentSha256) throw new Error('Invalid checkpoint content digest');
	return checkpoint.identity;
}

export function git(root, ...args) {
	return execFileSync('git', ['-C', root, ...args], { encoding: 'utf8' }).trim();
}
