import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { runInNewContext } from 'node:vm';

const workflow = (name) =>
	readFile(new URL(`../.github/workflows/${name}.yml`, import.meta.url), 'utf8');

// Execute the shipped shell steps, including their real Git operations, without Actions or delivery credentials.
function runStep(source, name) {
	const start = source.indexOf(`      - name: ${name}\n`);
	assert.notEqual(start, -1, name);
	const step = source.slice(start).split(/\n {6}- /)[0];
	const body = step.match(/ {8}run: \|\n((?: {10}[^\n]*(?:\n|$)|\n)*)/);
	assert.ok(body, name);
	return body[1].replace(/^ {10}/gm, '');
}

function jobSource(source, name) {
	const start = source.indexOf(`\n  ${name}:\n`);
	assert.notEqual(start, -1, name);
	return source.slice(start).split(/\n {2}(?=[\w-]+:)/)[1];
}

// Evaluate these guards with Actions' implicit success() unless a status function overrides it.
// https://docs.github.com/en/actions/reference/workflows-and-actions/expressions#status-check-functions
function jobAllowed(source, name, context, ancestorResults = []) {
	const condition = jobSource(source, name).match(/ {4}if: >-\n((?: {6}[^\n]+\n)+)/);
	assert.ok(condition, name);
	const expression = condition[1].trim();
	const hasStatusCheck = /\b(always|cancelled|failure|success)\s*\(/.test(expression);
	return runInNewContext(hasStatusCheck ? expression : `success() && (${expression})`, {
		...context,
		success: () => ancestorResults.every((result) => result === 'success'),
		cancelled: () => ancestorResults.includes('cancelled')
	});
}

async function fixture(t) {
	const root = await mkdtemp(join(tmpdir(), 'ditherette-release-workflow-'));
	t.after(() => rm(root, { recursive: true, force: true }));
	const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
	git('init', '-b', 'main');
	git('config', 'user.name', 'Release test');
	git('config', 'user.email', 'release@example.test');
	git('config', 'commit.gpgsign', 'false');
	await writeFile(join(root, '.gitignore'), 'temporary/\n');
	await mkdir(join(root, 'temporary'));
	git('add', '.');
	git('commit', '-m', 'fixture');
	return { root, git };
}

for (const [step, operation] of [
	['Prepare the exact release tarball', 'prepare'],
	['Verify source, artifact, sizes, and recorded release holds', 'publish-check']
]) {
	test(`npm ${operation} removes the cache override and rejects custom build flags`, async (t) => {
		const { root } = await fixture(t);
		const bin = join(root, 'temporary/bin');
		await mkdir(bin);
		const guard = new URL('./prepare-public-benchmark.mjs', import.meta.url).href;
		// Probe the real guard at each release invocation without building or publishing a package.
		await writeFile(
			join(bin, 'node'),
			`#!${process.execPath}
const assert = require('node:assert/strict');
const args = process.argv.slice(2);
if (args[0] === 'scripts/release-plan.mjs') {
  assert.deepEqual(args, ['scripts/release-plan.mjs', 'current', 'npm']);
  process.exit(0);
}
assert.deepEqual(args.slice(0, 2), ['packages/ditherette/scripts/release.mjs', ${JSON.stringify(operation)}]);
import(${JSON.stringify(guard)}).then(({ assertBuildEnvironment }) => {
  assertBuildEnvironment(process.env);
  console.log('Checked ' + args[1]);
});
`,
			{ mode: 0o755 }
		);
		const script = runStep(await workflow('package-publish'), step);
		const run = (flags) =>
			spawnSync('bash', ['-euo', 'pipefail', '-c', script], {
				cwd: root,
				encoding: 'utf8',
				env: {
					...process.env,
					PATH: `${bin}:${process.env.PATH}`,
					GITHUB_WORKSPACE: root,
					CARGO_INCREMENTAL: '0',
					RUSTFLAGS: flags || undefined
				}
			});
		const ordinary = run('');
		assert.equal(ordinary.status, 0, ordinary.stdout + ordinary.stderr);
		assert.match(ordinary.stdout, new RegExp(`Checked ${operation}`));
		const overridden = run('-C opt-level=0');
		assert.notEqual(overridden.status, 0);
		assert.match(overridden.stderr, /Unset build overrides before preparation: RUSTFLAGS/);
	});
}

test('v1 publication exception rejects other commits before artifact verification', async (t) => {
	const { root } = await fixture(t);
	const bin = join(root, 'temporary/bin');
	await mkdir(bin);
	await mkdir(join(root, 'packages/ditherette'), { recursive: true });
	await writeFile(join(root, 'packages/ditherette/package.json'), '{"version":"1.0.0"}');
	await writeFile(join(bin, 'node'), '#!/bin/sh\nprintf "%s\\n" "$*"\n', { mode: 0o755 });
	await writeFile(join(bin, 'git'), '#!/bin/sh\nexit "${DIFF_STATUS:-0}"\n', { mode: 0o755 });
	const script = runStep(
		await workflow('package-publish'),
		'Verify source, artifact, sizes, and recorded release holds'
	);
	const run = (sha, diffStatus = '0') =>
		spawnSync('bash', ['-euo', 'pipefail', '-c', script], {
			cwd: root,
			encoding: 'utf8',
			env: {
				...process.env,
				PATH: `${bin}:${process.env.PATH}`,
				OVERRIDE_V1_RELEASE_HOLDS: 'true',
				RELEASE_AUTHORIZATION_SHA: sha,
				DITHERETTE_RELEASE_SHA: 'workflow-source',
				GITHUB_SHA: 'workflow-source',
				DIFF_STATUS: diffStatus,
				GITHUB_WORKSPACE: root
			}
		});
	const result = run('another-release');
	assert.notEqual(result.status, 0, result.stdout + result.stderr);
	assert.doesNotMatch(result.stdout, /release\.mjs verify/);
	const approved = run('259b6258d5060523699d5e0b1d7bf24eb1c4e9d7');
	assert.equal(approved.status, 0, approved.stdout + approved.stderr);
	assert.match(approved.stdout, /release\.mjs verify/);
	const changedSource = run('259b6258d5060523699d5e0b1d7bf24eb1c4e9d7', '1');
	assert.notEqual(changedSource.status, 0);
	assert.doesNotMatch(changedSource.stdout, /release\.mjs verify/);
});

test('npm publication requires source and workflow identity to agree', async (t) => {
	const { root, git } = await fixture(t);
	const sha = git('rev-parse', 'HEAD');
	const bin = join(root, 'temporary/bin');
	await mkdir(bin);
	await writeFile(join(bin, 'npm'), '#!/bin/sh\nprintf "%s\\n" "$GITHUB_SHA"\n', { mode: 0o755 });
	const script = runStep(
		await workflow('package-publish'),
		'Publish the tested tarball through trusted publishing'
	);
	const run = (workflowSha) =>
		spawnSync('bash', ['-euo', 'pipefail', '-c', script], {
			cwd: root,
			encoding: 'utf8',
			env: {
				...process.env,
				PATH: `${bin}:${process.env.PATH}`,
				PUBLISH_SHA: sha,
				GITHUB_SHA: workflowSha
			}
		});
	const rejected = run('newer-dispatch');
	assert.notEqual(rejected.status, 0);
	assert.equal(rejected.stdout, '');
	const result = run(sha);
	assert.equal(result.status, 0, result.stdout + result.stderr);
	assert.equal(result.stdout.trim(), sha);
});

test('validation fetches a squash-merged PR head and falls back when that ref is unavailable', async (t) => {
	const { root, git } = await fixture(t);
	const base = git('rev-parse', 'HEAD');
	await writeFile(join(root, 'release.txt'), 'release\n');
	git('add', '.');
	git('commit', '-m', 'release PR');
	const head = git('rev-parse', 'HEAD');
	const sha = git('commit-tree', `${head}^{tree}`, '-p', base, '-m', 'squash release');
	const origin = join(root, 'temporary/origin.git');
	execFileSync('git', ['init', '--bare', origin]);
	git('remote', 'add', 'origin', origin);
	git('push', 'origin', `${sha}:refs/heads/main`, `${head}:refs/pull/123/head`);
	const repository = 'mia-cx/ditherette';
	const pullRequest = {
		number: 123,
		merged_at: '2026-10-11T00:00:00Z',
		merge_commit_sha: sha,
		base: { ref: 'main', repo: { full_name: repository } },
		head: { ref: 'changeset-release/main', sha: head, repo: { full_name: repository } }
	};
	const bin = join(root, 'temporary/bin');
	await mkdir(bin);
	await writeFile(
		join(bin, 'gh'),
		'#!/usr/bin/env node\nconsole.log(process.env.RELEASE_RUNS);\n',
		{
			mode: 0o755
		}
	);
	const env = {
		...process.env,
		PATH: `${bin}:${process.env.PATH}`,
		GITHUB_REPOSITORY: repository,
		GITHUB_SHA: sha,
		DITHERETTE_RELEASE_SHA: sha,
		RELEASE_RUNS: JSON.stringify({
			workflow_runs: [
				{
					event: 'pull_request',
					path: '.github/workflows/release-validation.yml',
					head_sha: head,
					pull_requests: [{ number: 123 }],
					status: 'completed',
					conclusion: 'success'
				}
			]
		})
	};
	for (const available of [true, false]) {
		if (!available) git('push', 'origin', ':refs/pull/123/head');
		const runner = join(root, `temporary/runner-${available}`);
		// A full main-only checkout omits the PR commit after a squash and branch deletion.
		execFileSync('git', [
			'clone',
			'--no-local',
			'--single-branch',
			'--branch',
			'main',
			origin,
			runner
		]);
		assert.notEqual(spawnSync('git', ['cat-file', '-e', head], { cwd: runner }).status, 0);
		const output = join(runner, 'actions-output');
		const run = spawnSync(
			process.execPath,
			[fileURLToPath(new URL('./release-plan.mjs', import.meta.url)), 'validated'],
			{
				cwd: runner,
				encoding: 'utf8',
				input: JSON.stringify([pullRequest]),
				env: { ...env, GITHUB_OUTPUT: output }
			}
		);
		assert.equal(run.status, 0, run.stdout + run.stderr);
		assert.equal(await readFile(output, 'utf8'), `validated=${available}\n`);
		if (!available) assert.match(run.stderr, /requiring fresh validation/);
	}
});

test('CI result fails when change detection fails or is cancelled and selected tests are skipped', async () => {
	const source = await workflow('ci');
	const script = runStep(source, 'Report the aggregate result');
	for (const result of ['failure', 'cancelled']) {
		const run = spawnSync('bash', ['-euo', 'pipefail', '-c', script], {
			encoding: 'utf8',
			env: {
				...process.env,
				CHANGES: result,
				LINT: 'success',
				CHANGESET: 'success',
				WEB: 'skipped',
				ENGINE: 'skipped',
				TOOLING: 'skipped'
			}
		});
		assert.equal(run.status, 1, run.stdout + run.stderr);
	}
	assert.ok(source.includes('needs: [changes, lint, changeset, web, engine, release-tooling]'));
});

test('helper-only edits select their CI tests while documentation skips them', async (t) => {
	const { root, git } = await fixture(t);
	const script = runStep(await workflow('ci'), 'Detect changed areas');
	for (const [path, expected, web = false] of [
		['scripts/check-changesets.mjs', true],
		['scripts/check-changesets.test.mjs', true],
		['scripts/changelog-section.mjs', true],
		['scripts/changelog-section.test.mjs', true],
		['scripts/prepare-public-benchmark.mjs', true],
		['scripts/prepare-public-benchmark.test.mjs', true],
		['scripts/typography-compatibility.mjs', false, true],
		['scripts/typography-compatibility.test.mjs', false, true],
		['scripts/fixtures/typography-compatibility.json', false, true],
		['docs/note.md', false]
	]) {
		const base = git('rev-parse', 'HEAD');
		await mkdir(dirname(join(root, path)), { recursive: true });
		await writeFile(join(root, path), 'fixture\n');
		git('add', path);
		git('commit', '-m', path);
		const output = join(root, 'temporary/output');
		await writeFile(output, '');
		execFileSync('bash', ['-euo', 'pipefail', '-c', script], {
			cwd: root,
			env: {
				...process.env,
				EVENT: 'pull_request',
				BASE_SHA: base,
				RUNNER_TEMP: join(root, 'temporary'),
				GITHUB_OUTPUT: output
			}
		});
		assert.match(await readFile(output, 'utf8'), new RegExp(`^tooling=${expected}$`, 'm'), path);
		assert.match(await readFile(output, 'utf8'), new RegExp(`^web=${web}$`, 'm'), path);
	}
});

for (const [name, step, manifest, changelog, tag] of [
	[
		'package-publish',
		'Tag and release the published version',
		'packages/ditherette/package.json',
		'packages/ditherette/CHANGELOG.md',
		'ditherette@0.0.2'
	]
]) {
	test(`${name} recovers a missing GitHub release after pushing its tag`, async (t) => {
		const { root, git } = await fixture(t);
		await mkdir(dirname(join(root, manifest)), { recursive: true });
		await writeFile(join(root, manifest), JSON.stringify({ version: '0.0.2' }));
		await writeFile(join(root, changelog), '# Changelog\n\n## 0.0.2\n\nFixture release.\n');
		await mkdir(join(root, 'scripts'));
		await cp(
			new URL('./changelog-section.mjs', import.meta.url),
			join(root, 'scripts/changelog-section.mjs')
		);
		git('add', '.');
		git('commit', '-m', 'release');
		const sha = git('rev-parse', 'HEAD');
		const origin = join(root, 'temporary/origin.git');
		execFileSync('git', ['init', '--bare', origin]);
		git('remote', 'add', 'origin', origin);
		git('push', 'origin', 'HEAD:main');
		const bin = join(root, 'temporary/bin');
		await mkdir(bin);
		const record = join(root, 'temporary/created.json');
		await writeFile(
			join(bin, 'gh'),
			`#!/usr/bin/env node
const fs = require('node:fs');
const args = process.argv.slice(2);
if (args[0] !== 'release') process.exit(2);
if (args[1] === 'view') process.exit(fs.existsSync(process.env.RELEASE_RECORD) ? 0 : 1);
if (args[1] !== 'create' || process.env.FAIL_CREATE === 'true') process.exit(1);
fs.writeFileSync(process.env.RELEASE_RECORD, JSON.stringify({ args, notes: fs.readFileSync(args[args.indexOf('--notes-file') + 1], 'utf8') }));
`,
			{ mode: 0o755 }
		);
		const script = runStep(await workflow(name), step).replace(
			/\$\{\{ (?:needs\.version\.outputs\.sha|inputs\.sha) \}\}/g,
			sha
		);
		const run = (fail) =>
			spawnSync('bash', ['-euo', 'pipefail', '-c', script], {
				cwd: root,
				encoding: 'utf8',
				env: {
					...process.env,
					PATH: `${bin}:${process.env.PATH}`,
					RUNNER_TEMP: join(root, 'temporary'),
					RELEASE_RECORD: record,
					FAIL_CREATE: String(fail)
				}
			});
		assert.equal(run(true).status, 1);
		assert.equal(git('rev-list', '-n', '1', tag), sha);
		const retry = run(false);
		assert.equal(retry.status, 0, retry.stdout + retry.stderr);
		const release = JSON.parse(await readFile(record, 'utf8'));
		assert.equal(release.args[2], tag);
		assert.equal(release.args[release.args.indexOf('--target') + 1], sha);
		assert.equal(release.notes.trim(), 'Fixture release.');
		assert.equal(run(true).status, 0, 'an existing release must not be created again');
		git('tag', '--force', tag, `${sha}^`);
		const conflict = run(false);
		assert.equal(conflict.status, 1, conflict.stdout + conflict.stderr);
	});
}

test('main delivery requires successful CI and exact-commit conformance before production', async (t) => {
	const source = await workflow('ci');
	const validation = jobSource(source, 'production_validation');
	const delivery = jobSource(source, 'website');
	assert.match(validation, /needs: result\n/);
	assert.match(validation, /uses: \.\/\.github\/workflows\/release-validation\.yml\n/);
	assert.match(validation, /sha: \$\{\{ github\.sha \}\}/);
	assert.match(delivery, /needs: \[result, production_validation\]/);
	assert.match(delivery, /ref: \$\{\{ github\.sha \}\}/);
	assert.match(delivery, /environment: production\n/);
	assert.match(delivery, /group: ditherette-website\n\s+cancel-in-progress: false/);
	assert.doesNotMatch(await workflow('release'), /wrangler (deploy|d1)|environment: production/);
	const github = {
		repository: 'mia-cx/ditherette',
		ref: 'refs/heads/main',
		event_name: 'push'
	};
	const needs = {
		result: { result: 'success' },
		production_validation: { result: 'success' }
	};
	for (const [label, context, allowed] of [
		['ordinary main push', { github, needs }, true],
		[
			'manual main recovery',
			{ github: { ...github, event_name: 'workflow_dispatch' }, needs },
			true
		],
		['PR', { github: { ...github, event_name: 'pull_request' }, needs }, false],
		[
			'privileged PR event',
			{ github: { ...github, event_name: 'pull_request_target' }, needs },
			false
		],
		['fork', { github: { ...github, repository: 'fork/ditherette' }, needs }, false],
		['branch dispatch', { github: { ...github, ref: 'refs/heads/untrusted' }, needs }, false],
		...['failure', 'cancelled', 'skipped'].flatMap((result) => [
			[`CI ${result}`, { github, needs: { ...needs, result: { result } } }, false],
			[
				`conformance ${result}`,
				{ github, needs: { ...needs, production_validation: { result } } },
				false
			]
		])
	]) {
		await t.test(label, () => {
			assert.equal(jobAllowed(source, 'website', context), allowed);
			const validate = jobAllowed(source, 'production_validation', context);
			assert.equal(
				validate,
				context.needs.result.result === 'success' &&
					['push', 'workflow_dispatch'].includes(context.github.event_name) &&
					context.github.repository === github.repository &&
					context.github.ref === github.ref
			);
		});
	}
});

test('production jobs survive skipped ancestors but still reject cancellation and failed validation', async () => {
	const source = await workflow('ci');
	const context = {
		github: {
			repository: 'mia-cx/ditherette',
			ref: 'refs/heads/main',
			event_name: 'push'
		},
		needs: {
			result: { result: 'success' },
			production_validation: { result: 'success' }
		}
	};
	// Run38641062557 passed its aggregate after Changeset, Web and Engine intentionally skipped.
	const ancestors = ['success', 'success', 'skipped', 'success', 'skipped', 'skipped', 'success'];
	for (const job of ['production_validation', 'website']) {
		for (const event_name of ['push', 'workflow_dispatch']) {
			const current = { ...context, github: { ...context.github, event_name } };
			assert.equal(jobAllowed(source, job, current, ancestors), true, `${job}: ${event_name}`);
			assert.equal(jobAllowed(source, job, current, [...ancestors, 'cancelled']), false);
			for (const result of ['failure', 'cancelled', 'skipped']) {
				const rejected = { ...current, needs: { ...current.needs, result: { result } } };
				assert.equal(jobAllowed(source, job, rejected, ancestors), false);
			}
		}
	}
	for (const result of ['failure', 'cancelled', 'skipped']) {
		const rejected = {
			...context,
			needs: { ...context.needs, production_validation: { result } }
		};
		assert.equal(jobAllowed(source, 'website', rejected, ancestors), false);
	}
});

test('validation callers at the same commit cannot cancel each other; PR heads still supersede', async () => {
	const source = await workflow('release-validation');
	const concurrency = source.match(/\nconcurrency:\n([\s\S]*?)\njobs:/)?.[1];
	assert.ok(concurrency);
	const group = concurrency.match(/ {2}group: ([^\n]+)/)?.[1];
	assert.ok(group);
	assert.match(concurrency, /cancel-in-progress: true\n/);
	const key = (workflow, sha, number) =>
		group.replace(/\$\{\{\s*(.*?)\s*\}\}/g, (_, expression) =>
			runInNewContext(expression, {
				github: { workflow, sha, event: { pull_request: { number } } },
				inputs: { sha: number ? '' : sha }
			})
		);
	const sha = 'a'.repeat(40);
	assert.notEqual(key('CI', sha), key('Release', sha));
	assert.notEqual(key('CI', sha), key('Release validation', sha));
	assert.equal(key('Release validation', sha, 123), key('Release validation', 'b'.repeat(40), 123));
	assert.notEqual(key('Release validation', sha, 123), key('Release validation', sha, 124));
});

test('website shell deploys ordinary main, rejects older queued commits and stops on migration errors', async (t) => {
	const { root, git } = await fixture(t);
	const source = await workflow('ci');
	const reject = runStep(source, 'Reject superseded website commits');
	const deliver = runStep(source, 'Deliver the current website commit');
	const origin = join(root, 'temporary/origin.git');
	execFileSync('git', ['init', '--bare', origin]);
	git('remote', 'add', 'origin', origin);
	git('push', 'origin', 'HEAD:main');
	const sha = git('rev-parse', 'HEAD');
	const bin = join(root, 'temporary/bin');
	await mkdir(bin);
	const record = join(root, 'temporary/deliveries');
	await writeFile(
		join(bin, 'pnpm'),
		`#!${process.execPath}
const fs = require('node:fs');
const args = process.argv.slice(2);
fs.appendFileSync(process.env.DELIVERY_RECORD, JSON.stringify(args) + '\\n');
if (args.includes('migrations') && process.env.FAIL_MIGRATION === 'true') process.exit(1);
`,
		{ mode: 0o755 }
	);
	const env = {
		...process.env,
		PATH: `${bin}:${process.env.PATH}`,
		GITHUB_SHA: sha,
		CLOUDFLARE_API_TOKEN: 'fixture-token',
		DELIVERY_RECORD: record
	};
	const run = (script, overrides = {}) =>
		spawnSync('bash', ['-euo', 'pipefail', '-c', script], {
			cwd: root,
			encoding: 'utf8',
			env: { ...env, ...overrides }
		});
	assert.equal(run(reject).status, 0);
	const ordinary = run(deliver);
	assert.equal(ordinary.status, 0, ordinary.stdout + ordinary.stderr);
	assert.deepEqual((await readFile(record, 'utf8')).trim().split('\n').map(JSON.parse), [
		['exec', 'wrangler', 'd1', 'migrations', 'apply', 'EVENTS', '--remote', '--env', 'production'],
		['exec', 'wrangler', 'deploy', '--env', 'production']
	]);
	await writeFile(record, '');
	const failed = run(deliver, { FAIL_MIGRATION: 'true' });
	assert.equal(failed.status, 1, failed.stdout + failed.stderr);
	assert.equal((await readFile(record, 'utf8')).trim().split('\n').length, 1);
	await writeFile(record, '');
	assert.equal(run(deliver, { CLOUDFLARE_API_TOKEN: '' }).status, 1);
	assert.equal(run(deliver, { GITHUB_SHA: 'a'.repeat(40) }).status, 1);
	assert.equal(await readFile(record, 'utf8'), '');
	// The version need not change for a newer main commit to supersede a queued build.
	await writeFile(join(root, 'ordinary-change.txt'), 'newer website\n');
	git('add', '.');
	git('commit', '-m', 'ordinary main change');
	const latest = git('rev-parse', 'HEAD');
	git('push', 'origin', 'HEAD:main');
	git('checkout', '--detach', sha);
	// The first check caught a stale queued candidate. The second also catches a push during its build.
	for (const script of [reject, deliver]) {
		const stale = run(script);
		assert.equal(stale.status, 1, stale.stdout + stale.stderr);
		assert.match(stale.stdout, /superseded/);
	}
	assert.equal(await readFile(record, 'utf8'), '', 'stale commits must not migrate or deploy');
	git('checkout', '--detach', latest);
	assert.equal(run(deliver, { GITHUB_SHA: latest }).status, 0);
	git('remote', 'remove', 'origin');
	await writeFile(record, '');
	assert.notEqual(run(deliver, { GITHUB_SHA: latest }).status, 0);
	assert.equal(await readFile(record, 'utf8'), '', 'a failed freshness fetch must stop delivery');
});
