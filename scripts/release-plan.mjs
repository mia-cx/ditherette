import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { appendFileSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { releaseChannel } from '../packages/ditherette/scripts/release-contract.mjs';

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
		assert.match(version, /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-rc\.(0|[1-9]\d*))?$/);
		return version.split('-')[0].split('.').map(BigInt);
	};
	const previous = parse(before);
	const current = parse(after);
	assert.ok(!after.includes('-'), 'Automatic releases must not be release candidates.');
	const first = current.findIndex((part, index) => part !== previous[index]);
	if (first === -1) return before.includes('-rc.');
	assert.ok(current[first] > previous[first], 'Release versions must increase.');
	return true;
}

export function releasePlan(pullRequests, context, versions, pending = []) {
	assert.equal(context.repository, 'mia-cx/ditherette');
	assert.match(context.sha, /^[a-f0-9]{40}$/);
	const release =
		context.event === 'push' &&
		context.ref === 'refs/heads/main' &&
		isReleaseMerge(pullRequests, context) &&
		releaseChannel(versions.npm[1]) === 'latest';
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

/** The head commit of the merged release pull request, whose tree must equal the merge's. */
export function releasePullHead(pullRequests, context) {
	const pr = pullRequests.find(
		(pr) =>
			typeof pr.merged_at === 'string' &&
			pr.merge_commit_sha === context.sha &&
			pr.base?.ref === 'main' &&
			pr.base?.repo?.full_name === context.repository &&
			pr.head?.ref === 'changeset-release/main' &&
			pr.head?.repo?.full_name === context.repository
	);
	return pr?.head?.sha ?? null;
}

/** The release PR is validated only when its head passed the approval-gated suite. */
export function releaseValidated(checkRuns) {
	return checkRuns.some(
		(run) =>
			run.name === 'Release validation' &&
			run.app?.slug === 'github-actions' &&
			run.status === 'completed' &&
			run.conclusion === 'success'
	);
}

/** Refuse reruns after another release changed either target version. */
export function requireCurrentVersion(released, current) {
	assert.equal(released, current, 'A newer release exists on main. Do not replay this release.');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	// DITHERETTE_RELEASE_SHA names the release merge; GITHUB_SHA names the runner's commit.
	// DITHERETTE_RELEASE_RETRY replays a verified release merge from any event; the merge
	// check below still fails closed for commits that are not release merges.
	const sha = process.env.DITHERETTE_RELEASE_SHA ?? process.env.GITHUB_SHA;
	const repository = process.env.GITHUB_REPOSITORY;
	const retry = process.env.DITHERETTE_RELEASE_RETRY === 'true';
	const event = retry ? 'push' : process.env.GITHUB_EVENT_NAME;
	const ref = retry ? 'refs/heads/main' : process.env.GITHUB_REF;
	assert.match(sha ?? '', /^[a-f0-9]{40}$/);
	const mode = process.argv[2];
	if (mode === 'validated') {
		const pullRequests = JSON.parse(readFileSync(0, 'utf8')).flat();
		const head = releasePullHead(pullRequests, { sha, repository });
		let validated = false;
		if (head) {
			const { check_runs: checkRuns } = JSON.parse(
				execFileSync(
					'gh',
					[
						'api',
						`repos/${repository}/commits/${head}/check-runs?check_name=Release%20validation&per_page=100`
					],
					{ encoding: 'utf8' }
				)
			);
			validated =
				releaseValidated(checkRuns) &&
				git('rev-parse', `${sha}^{tree}`) === git('rev-parse', `${head}^{tree}`);
		}
		console.log(`validated=${validated}`);
		if (process.env.GITHUB_OUTPUT)
			appendFileSync(process.env.GITHUB_OUTPUT, `validated=${validated}\n`);
	} else if (mode === 'current') {
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
			// SHA checkouts in Actions omit the local branch required by Changesets.
			// Create it only in detached CI checkouts; never move an existing branch.
			if (
				process.env.GITHUB_ACTIONS === 'true' &&
				git('rev-parse', '--abbrev-ref', 'HEAD') === 'HEAD' &&
				git('for-each-ref', '--format=%(refname)', 'refs/heads/main') === ''
			) {
				assert.equal(git('rev-parse', 'HEAD'), sha);
				git('update-ref', 'refs/heads/main', sha, '0'.repeat(40));
			}
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
