import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
	changedChangesets,
	checkChangesets,
	isUserFacing,
	parseChangeset,
	validateChangeset
} from './check-changesets.mjs';

const valid = `---
'ditherette': patch
'ditherette-web': minor
---

A note worth releasing.
`;

test('parseChangeset splits frontmatter from the note', () => {
	const { frontmatter, body } = parseChangeset(valid);
	assert.match(frontmatter, /ditherette/);
	assert.equal(body, 'A note worth releasing.');
});

test('validateChangeset accepts both released packages and every bump kind', () => {
	validateChangeset(valid, 'x.md');
	validateChangeset(`---\nditherette-web: major\n---\nShip it.\n`, 'x.md');
});

test('validateChangeset rejects malformed files', () => {
	for (const [content, message] of [
		['no frontmatter', 'expected frontmatter'],
		['---\n\n---\nnote\n', 'declare a patch, minor, or major'],
		["---\n'ditherette-wasm': patch\n---\nnote\n", 'is not a released package'],
		["---\n'ditherette': patch\n---\n\n", 'release note is empty']
	]) {
		assert.throws(() => validateChangeset(content, 'bad.md'), new RegExp(message));
	}
});

test('changedChangesets keeps .changeset markdown but not README or config', () => {
	assert.deepEqual(
		changedChangesets([
			'.changeset/new-note.md',
			'.changeset/README.md',
			'.changeset/config.json',
			'src/lib.ts'
		]),
		['.changeset/new-note.md']
	);
});

test('isUserFacing covers the shipping paths only', () => {
	assert.ok(isUserFacing('src/lib/app.ts'));
	assert.ok(isUserFacing('static/favicon.png'));
	assert.ok(isUserFacing('packages/ditherette/src/types.ts'));
	assert.ok(isUserFacing('crates/ditherette-wasm/src/lib.rs'));
	assert.ok(isUserFacing('migrations/0001.sql'));
	assert.ok(!isUserFacing('.github/workflows/ci.yml'));
	assert.ok(!isUserFacing('docs/releases.md'));
	assert.ok(!isUserFacing('packages/ditherette/tests/effects.test.mjs'));
});

test('checkChangesets passes a valid changeset over a user-facing diff', () => {
	checkChangesets({
		files: ['src/lib/app.ts', '.changeset/note.md'],
		changesets: { '.changeset/note.md': valid }
	});
});

test('checkChangesets fails a user-facing diff without a changeset', () => {
	assert.throws(
		() => checkChangesets({ files: ['src/lib/app.ts'], changesets: {} }),
		/without a changeset/
	);
});

test('checkChangesets lets the `no changeset` label through', () => {
	checkChangesets({ files: ['src/lib/app.ts'], changesets: {}, labels: ['no changeset'] });
});

test('checkChangesets fails a malformed added changeset', () => {
	assert.throws(
		() =>
			checkChangesets({
				files: ['.changeset/bad.md'],
				changesets: { '.changeset/bad.md': 'not a changeset' }
			}),
		/expected frontmatter/
	);
});

test('checkChangesets ignores non-user-facing diffs', () => {
	checkChangesets({ files: ['.github/workflows/ci.yml'], changesets: {} });
});
