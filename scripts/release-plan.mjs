import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { appendFileSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const releasePackages = {
	npm: 'packages/ditherette/package.json',
	web: 'package.json'
};

/** The exact merge of this repository's Changesets PR authorizes release processing. */
export function isReleaseMerge(pullRequests, { sha, repository }) {
	return pullRequests.some(
		(pr) =>
			typeof pr.merged_at === 'string' &&
			pr.merge_commit_sha === sha &&
			pr.base?.ref === 'main' &&
			pr.base?.repo?.full_name === repository &&
			pr.head?.ref === 'changeset-release/main' &&
			pr.head?.repo?.full_name === repository
	);
}

/** Manifest edits only release an app when its stable version increases. */
export function versionIncreased(before, after) {
	const parse = (version) => {
		assert.match(version, /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/);
		return version.split('.').map(BigInt);
	};
	const previous = parse(before);
	const current = parse(after);
	const first = current.findIndex((part, index) => part !== previous[index]);
	if (first === -1) return false;
	assert.ok(current[first] > previous[first], 'Release versions must increase.');
	return true;
}

export function releasePlan(pullRequests, context, versions, pending = []) {
	assert.equal(context.repository, 'mia-cx/ditherette');
	assert.match(context.sha, /^[a-f0-9]{40}$/);
	const release =
		context.event === 'push' &&
		context.ref === 'refs/heads/main' &&
		isReleaseMerge(pullRequests, context);
	return Object.fromEntries([
		['release', release],
		...Object.entries(versions).map(([name, [before, after]]) => [
			name,
			release && !pending.includes(name) && versionIncreased(before, after)
		])
	]);
}

const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();
const manifestAt = (revision, path) => JSON.parse(git('show', `${revision}:${path}`));

/** Refuse reruns after another release changed either target version. */
export function requireCurrentVersion(released, current) {
	assert.equal(released, current, 'A newer release exists on main. Do not replay this release.');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const {
		GITHUB_SHA: sha,
		GITHUB_REPOSITORY: repository,
		GITHUB_EVENT_NAME: event,
		GITHUB_REF: ref
	} = process.env;
	assert.match(sha ?? '', /^[a-f0-9]{40}$/);
	const mode = process.argv[2];
	if (mode === 'current') {
		const path = releasePackages[process.argv[3]];
		assert.ok(path, 'Expected npm or web release target.');
		git('fetch', '--no-tags', 'origin', '+refs/heads/main:refs/remotes/origin/main');
		requireCurrentVersion(manifestAt(sha, path).version, manifestAt('origin/main', path).version);
	} else {
		assert.equal(mode, undefined);
		const pullRequests = JSON.parse(readFileSync(0, 'utf8')).flat();
		const versions = Object.fromEntries(
			Object.entries(releasePackages).map(([name, path]) => [
				name,
				[manifestAt(`${sha}^1`, path).version, manifestAt(sha, path).version]
			])
		);
		const temporary = mkdtempSync(join(tmpdir(), 'ditherette-release-plan-'));
		let pending;
		try {
			const status = join(temporary, 'status.json');
			execFileSync(
				process.execPath,
				['node_modules/@changesets/cli/bin.js', 'status', '--output', status],
				{ stdio: 'inherit' }
			);
			const { releases } = JSON.parse(readFileSync(status, 'utf8'));
			pending = releases.map(({ name }) =>
				name === 'ditherette' ? 'npm' : name === 'ditherette-web' ? 'web' : name
			);
		} finally {
			rmSync(temporary, { recursive: true, force: true });
		}
		const plan = {
			...releasePlan(pullRequests, { sha, repository, event, ref }, versions, pending),
			pending: pending.length > 0
		};
		for (const [name, value] of Object.entries(plan)) {
			console.log(`${name}=${value}`);
			if (process.env.GITHUB_OUTPUT)
				appendFileSync(process.env.GITHUB_OUTPUT, `${name}=${value}\n`);
		}
	}
}
