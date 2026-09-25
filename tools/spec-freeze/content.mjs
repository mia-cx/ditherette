import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { lstatSync, readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';

export const CRATE = 'crates/ditherette-wasm';
export const FROZEN_ROOTS = [`${CRATE}/src/spec`, `${CRATE}/src/image`];
export const GENERATOR = `${CRATE}/examples/generate_blue_noise.rs`;
export const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

function checkedPath(root, relative) {
	let path = resolve(root);
	if (lstatSync(path).isSymbolicLink()) throw new Error('Symlink is forbidden: .');
	for (const component of relative.split('/')) {
		path = join(path, component);
		if (lstatSync(path).isSymbolicLink()) throw new Error(`Symlink is forbidden: ${relative}`);
	}
	return path;
}

/** Include untracked additions and file modes; never follow symlinks. */
export function files(root, relative) {
	const path = checkedPath(root, relative);
	const stat = lstatSync(path);
	if (stat.isFile()) return [relative];
	if (!stat.isDirectory()) throw new Error(`Unsupported file type: ${relative}`);
	return readdirSync(path)
		.sort()
		.flatMap((name) => files(root, `${relative}/${name}`));
}

export function inventory(root, paths) {
	return paths
		.flatMap((path) => files(root, path))
		.sort()
		.map((path) => {
			const full = checkedPath(root, path);
			return {
				path,
				mode: lstatSync(full).mode & 0o111 ? '100755' : '100644',
				sha256: sha256(readFileSync(full))
			};
		});
}

export function contentDigest(entries) {
	return sha256(entries.map(({ path, mode, sha256 }) => `${mode} ${sha256} ${path}\n`).join(''));
}

/** The only v1 file an extension may replace: it registers the new reference modules. */
export const EXTENSION_ROOT = `${CRATE}/src/spec/mod.rs`;

/** Apply one recorded change set, checking each recorded base before replacing it. */
function applyChanges(expected, changes, label, permitted = () => true) {
	const changed = new Set();
	for (const change of changes) {
		if (changed.has(change.path)) throw new Error(`Duplicate checkpoint ${label}: ${change.path}`);
		changed.add(change.path);
		if (!permitted(change))
			throw new Error(`Checkpoint ${label} cannot change v1 file: ${change.path}`);
		const before = expected.get(change.path) ?? null;
		if (JSON.stringify(before) !== JSON.stringify(change.before))
			throw new Error(`Invalid checkpoint ${label} base: ${change.path}`);
		if (change.after === null) expected.delete(change.path);
		else {
			if (change.after.path !== change.path)
				throw new Error(`Invalid checkpoint ${label} path: ${change.path}`);
			expected.set(change.path, change.after);
		}
	}
	return [...expected.values()].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

/** The amended v1 closure, before any extension. */
export function v1Files(checkpoint) {
	if (contentDigest(checkpoint.files) !== checkpoint.contentSha256)
		throw new Error('Invalid original checkpoint content digest');
	if (!checkpoint.amendment) return checkpoint.files;
	const files = applyChanges(
		new Map(checkpoint.files.map((entry) => [entry.path, entry])),
		checkpoint.amendment.changes,
		'amendment'
	);
	if (contentDigest(files) !== checkpoint.amendment.contentSha256)
		throw new Error('Invalid amended checkpoint content digest');
	return files;
}

export function expectedFiles(checkpoint) {
	let files = v1Files(checkpoint);
	// Extensions add later reference domains. They keep every v1 byte except the
	// module registration root, so v1 conformance identities stay valid.
	const v1 = new Set(files.map(({ path }) => path));
	for (const extension of checkpoint.extensions ?? []) {
		files = applyChanges(
			new Map(files.map((entry) => [entry.path, entry])),
			extension.changes,
			`extension ${extension.name}`,
			({ path }) => path === EXTENSION_ROOT || !v1.has(path)
		);
		if (contentDigest(files) !== extension.contentSha256)
			throw new Error(`Invalid checkpoint extension digest: ${extension.name}`);
	}
	return files;
}

/**
 * An extended `spec/mod.rs` must be the v1 bytes plus `pub mod <name>;` lines for
 * names v1 does not already use. Anything else could reroute a v1 module.
 */
export function verifyExtensionRoot(root, v1) {
	const recorded = v1.find(({ path }) => path === EXTENSION_ROOT);
	const prefix = `${CRATE}/src/spec/`;
	const names = new Set(
		v1
			.filter(({ path }) => path.startsWith(prefix))
			.map(({ path }) =>
				path
					.slice(prefix.length)
					.split('/')[0]
					.replace(/\.(rs|md)$/, '')
			)
	);
	const lines = readFileSync(checkedPath(root, EXTENSION_ROOT), 'utf8').split('\n');
	const original = lines.filter((line) => {
		const name = /^pub mod ([a-z_][a-z0-9_]*);$/.exec(line)?.[1];
		return !name || names.has(name);
	});
	if (sha256(original.join('\n')) !== recorded.sha256)
		throw new Error(`Extension root may only add module declarations: ${EXTENSION_ROOT}`);
}

/** Compare against recorded bytes, never a merge base or the current parent. */
export function verifyContent(root, checkpoint) {
	const actual = inventory(root, [...FROZEN_ROOTS, GENERATOR]);
	const recorded = expectedFiles(checkpoint);
	if (checkpoint.extensions?.length) verifyExtensionRoot(root, v1Files(checkpoint));
	if (JSON.stringify(actual) !== JSON.stringify(recorded)) {
		const expected = new Map(recorded.map((entry) => [entry.path, entry]));
		const found = new Map(actual.map((entry) => [entry.path, entry]));
		const changed = [...new Set([...expected.keys(), ...found.keys()])].filter(
			(path) => JSON.stringify(expected.get(path)) !== JSON.stringify(found.get(path))
		);
		throw new Error(`Frozen content changed:\n${changed.join('\n')}`);
	}
	return checkpoint.amendment?.identity ?? checkpoint.identity;
}

export function git(root, ...args) {
	return execFileSync('git', ['-C', root, ...args], { encoding: 'utf8' }).trim();
}
