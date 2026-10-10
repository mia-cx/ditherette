import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Every package a changeset may declare a bump for; everything else is ignored by releases. */
export const RELEASE_PACKAGES = new Set(['ditherette', 'ditherette-web']);

/** Paths whose changes always ship to users, so they need a recorded release note. */
export const USER_FACING = [
	/^src\//,
	/^static\//,
	/^packages\/ditherette\/src\//,
	/^crates\/ditherette-wasm\/src\//,
	/^migrations\//
];

const DECLARATION = /^['"]?([^'":\s]+)['"]?:\s*(patch|minor|major)\s*$/gm;

/** Split a changeset file into its frontmatter declarations and trimmed note body. */
export function parseChangeset(content, path = 'changeset.md') {
	const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/.exec(content);
	assert.ok(match !== null, `${path}: expected frontmatter between '---' lines`);
	return { frontmatter: match[1], body: match[2].trim() };
}

/** Validate one changeset file; throws with the offending path on every problem. */
export function validateChangeset(content, path = 'changeset.md') {
	const { frontmatter, body } = parseChangeset(content, path);
	const declarations = [...frontmatter.matchAll(DECLARATION)];
	assert.ok(declarations.length > 0, `${path}: declare a patch, minor, or major bump`);
	for (const [, name] of declarations) {
		assert.ok(
			RELEASE_PACKAGES.has(name),
			`${path}: '${name}' is not a released package; use ${[...RELEASE_PACKAGES].join(' or ')}`
		);
	}
	assert.notEqual(body.length, 0, `${path}: the release note is empty`);
}

/** Every changeset file in a diff: `.changeset/*.md` except the README and config. */
export function changedChangesets(files) {
	return files.filter(
		(file) =>
			/^\.changeset\/[^/]+\.md$/.test(file) &&
			!['README.md', 'config.json'].includes(basename(file))
	);
}

/** Whether a changed path always ships to users. */
export function isUserFacing(file) {
	return USER_FACING.some((pattern) => pattern.test(file));
}

/**
 * The whole gate. `files` are paths changed vs the PR base; `changesets` is a map of the
 * added/modified `.changeset/*.md` file names to their contents; `labels` are the PR's labels.
 */
export function checkChangesets({ files, changesets, labels = [] }) {
	for (const [path, content] of Object.entries(changesets)) {
		validateChangeset(content, path);
	}
	if (changesets && Object.keys(changesets).length > 0) return;
	if (files.some(isUserFacing) && !labels.includes('no changeset')) {
		throw new Error(
			'This change affects user-facing code without a changeset. ' +
				'Run `pnpm changeset` and commit the note, or add the label `no changeset`.'
		);
	}
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const [base] = process.argv.slice(2);
	if (!base) {
		console.error('usage: check-changesets <base-ref>');
		process.exit(2);
	}
	const git = (...args) =>
		execFileSync('git', args, { cwd: resolve(process.argv[1], '../..'), encoding: 'utf8' });
	const mergeBase = git('merge-base', base, 'HEAD').trim();
	const diff = (...args) =>
		git('diff', '--name-only', ...args)
			.split('\n')
			.filter(Boolean);
	const files = diff(`${mergeBase}...HEAD`);
	const changesets = {};
	for (const file of diff('--diff-filter=AMR', `${mergeBase}...HEAD`)) {
		if (changedChangesets([file]).length)
			changesets[file] = readFileSync(resolve(process.argv[1], '../../', file), 'utf8');
	}
	let labels = [];
	if (process.env.GITHUB_EVENT_PATH) {
		labels =
			JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8')).pull_request?.labels?.map(
				(label) => label.name
			) ?? [];
	}
	try {
		checkChangesets({ files, changesets, labels });
	} catch (error) {
		console.error(`::error::${error instanceof Error ? error.message : error}`);
		process.exit(1);
	}
}
