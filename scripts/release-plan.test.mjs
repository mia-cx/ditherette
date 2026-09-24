import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import {
	isReleaseMerge,
	releasePlan,
	requireCurrentVersion,
	versionIncreased
} from './release-plan.mjs';
import { alreadyPublished } from './release-registry.mjs';

const context = {
	sha: 'a'.repeat(40),
	repository: 'mia-cx/ditherette',
	event: 'push',
	ref: 'refs/heads/main'
};
const merged = {
	merged_at: '2026-09-24T00:00:00Z',
	merge_commit_sha: context.sha,
	base: { ref: 'main', repo: { full_name: context.repository } },
	head: { ref: 'changeset-release/main', repo: { full_name: context.repository } }
};

test('only the exact merged same-repository Changesets PR authorizes a release', () => {
	assert.equal(isReleaseMerge([merged], context), true);
	for (const pr of [
		{ ...merged, merged_at: null },
		{ ...merged, merge_commit_sha: 'b'.repeat(40) },
		{ ...merged, head: { ...merged.head, ref: 'feature' } },
		{ ...merged, head: { ...merged.head, repo: { full_name: 'fork/ditherette' } } },
		{ ...merged, base: { ...merged.base, ref: 'staging' } }
	])
		assert.equal(isReleaseMerge([pr], context), false);
});

test('only changed versions release their targets, ordinary pushes and manifest-only edits do not', () => {
	const versions = { npm: ['0.1.0', '0.1.1'], web: ['0.0.1', '0.0.1'] };
	assert.deepEqual(releasePlan([merged], context, versions), {
		release: true,
		npm: true,
		web: false
	});
	assert.deepEqual(releasePlan([], context, versions), { release: false, npm: false, web: false });
	assert.deepEqual(releasePlan([merged], context, versions, ['npm']), {
		release: true,
		npm: false,
		web: false
	});
	for (const change of [
		{ event: 'workflow_dispatch' },
		{ event: 'pull_request' },
		{ ref: 'refs/tags/v0.1.1' }
	])
		assert.deepEqual(releasePlan([merged], { ...context, ...change }, versions), {
			release: false,
			npm: false,
			web: false
		});
	assert.equal(versionIncreased('0.1.0', '0.1.0'), false);
	assert.equal(versionIncreased('0.1.0', '0.2.0'), true);
	assert.equal(versionIncreased('0.9.0', '1.0.0'), true);
	assert.equal(versionIncreased('0.1.0-rc.0', '0.1.0'), true);
	assert.equal(versionIncreased('0.1.0-rc.2', '0.2.0'), true);
	assert.throws(() => versionIncreased('0.1.0', '0.1.0-rc.0'));
	assert.deepEqual(
		releasePlan([merged], context, {
			npm: ['0.1.0', '0.1.0-rc.0'],
			web: ['0.0.1', '0.0.2']
		}),
		{ release: false, npm: false, web: false }
	);
	assert.throws(() => versionIncreased('0.2.0', '0.1.0'));
	assert.throws(() => versionIncreased('0.1.0', '0.1.1-beta.1'));
	requireCurrentVersion('0.1.0', '0.1.0');
	assert.throws(() => requireCurrentVersion('0.1.0', '0.1.1'), /newer release/);
});

test('publication reruns accept only identical archives and fail on registry errors', async () => {
	const archive = Buffer.from('tested package');
	const manifest = {
		name: 'ditherette',
		version: '0.1.1',
		dist: { integrity: `sha512-${createHash('sha512').update(archive).digest('base64')}` }
	};
	const request = async () => Response.json(manifest);
	assert.equal(await alreadyPublished('0.1.1', archive, request), true);
	assert.equal(
		await alreadyPublished('0.1.1', archive, async () => new Response(null, { status: 404 })),
		false
	);
	await assert.rejects(alreadyPublished('0.1.1', Buffer.from('different'), request), /differs/);
	await assert.rejects(
		alreadyPublished('0.1.1', archive, async () => new Response(null, { status: 503 })),
		/HTTP 503/
	);
	await assert.rejects(
		alreadyPublished('0.1.1', archive, async () => {
			throw new Error('network down');
		}),
		/network down/
	);
});
