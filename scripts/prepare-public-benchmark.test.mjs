import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, mkdir, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { cleanRevision, fileInventory, sourceInventory } from './prepare-public-benchmark.mjs';

test('source provenance rejects dirty input and records every tracked byte', async (t) => {
	const directory = await mkdtemp(path.join(tmpdir(), 'ditherette-source-provenance-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const git = (...args) => execFileSync('git', args, { cwd: directory, stdio: 'pipe' });
	git('init', '-q');
	await writeFile(path.join(directory, 'input'), 'first');
	git('add', 'input');
	git(
		'-c',
		'user.name=Fixture',
		'-c',
		'user.email=fixture@example.invalid',
		'commit',
		'-qm',
		'fixture'
	);
	assert.match(cleanRevision(directory), /^[0-9a-f]{40,64}$/);
	const before = await sourceInventory(directory);
	assert.equal(before.length, 1);
	assert.equal(before[0].path, 'input');
	assert.equal(before[0].bytes, 5);
	assert.equal(before[0].digest.length, 32);
	await writeFile(path.join(directory, 'input'), 'other');
	assert.throws(() => cleanRevision(directory), /clean source checkout/);
	assert.notDeepEqual((await sourceInventory(directory))[0].digest, before[0].digest);
});

test('built provenance sorts full trees and rejects symbolic links', async (t) => {
	const directory = await mkdtemp(path.join(tmpdir(), 'ditherette-built-provenance-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	await mkdir(path.join(directory, 'nested'));
	await writeFile(path.join(directory, 'z'), 'z');
	await writeFile(path.join(directory, 'nested/a'), 'a');
	assert.deepEqual(
		(await fileInventory(directory)).map(({ path }) => path),
		['nested/a', 'z']
	);
	await symlink('z', path.join(directory, 'alias'));
	await assert.rejects(fileInventory(directory), /not a regular file/);
});
