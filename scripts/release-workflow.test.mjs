import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

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

test('helper-only edits select the release tooling tests while documentation skips them', async (t) => {
	const { root, git } = await fixture(t);
	const script = runStep(await workflow('ci'), 'Detect changed areas');
	for (const [path, expected] of [
		['scripts/check-changesets.mjs', true],
		['scripts/check-changesets.test.mjs', true],
		['scripts/changelog-section.mjs', true],
		['scripts/changelog-section.test.mjs', true],
		['scripts/prepare-public-benchmark.mjs', true],
		['scripts/prepare-public-benchmark.test.mjs', true],
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
	}
});

for (const [name, step, manifest, changelog, tag] of [
	[
		'release',
		'Tag and release the deployed version',
		'package.json',
		'CHANGELOG.md',
		'ditherette-web@0.0.2'
	],
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
