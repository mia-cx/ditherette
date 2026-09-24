import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { replaceCrateVersion, synchronizeReleaseVersion } from './release-version.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));

test('crate synchronization changes only the named local package entry', () => {
	const source =
		'[[package]]\nname = "other"\nversion = "0.1.0"\n\n[[package]]\nname = "ditherette-wasm"\nversion = "0.1.0"\n';
	const result = replaceCrateVersion(source, '0.1.1', true);
	assert.match(result, /name = "other"\nversion = "0.1.0"/);
	assert.match(result, /name = "ditherette-wasm"\nversion = "0.1.1"/);
	assert.throws(() => replaceCrateVersion('', '0.1.1', true));
	assert.throws(() => replaceCrateVersion(source, '1.0.0', true));
});

for (const target of ['ditherette-web', 'ditherette']) {
	test(`real Changesets CLI versions ${target} with root website discovery and Rust synchronization`, async (t) => {
		const fixture = await mkdtemp(join(tmpdir(), 'ditherette-release-version-'));
		t.after(() => rm(fixture, { recursive: true, force: true }));
		for (const path of [
			'package.json',
			'pnpm-workspace.yaml',
			'.changeset/config.json',
			'packages/ditherette/package.json',
			'crates/ditherette-wasm/package.json',
			'crates/ditherette-wasm/Cargo.toml',
			'crates/ditherette-wasm/Cargo.lock',
			'crates/ditherette-bench/Cargo.lock'
		]) {
			await mkdir(dirname(join(fixture, path)), { recursive: true });
			await cp(join(root, path), join(fixture, path));
		}
		// Expose the CLI without making Changesets detect Prettier in this isolated fixture.
		await mkdir(join(fixture, 'node_modules/@changesets'), { recursive: true });
		await symlink(
			join(root, 'node_modules/@changesets/cli'),
			join(fixture, 'node_modules/@changesets/cli'),
			'dir'
		);
		await writeFile(join(fixture, '.gitignore'), 'node_modules\nstatus.json\noutputs\n');
		const beforeWeb = JSON.parse(await readFile(join(fixture, 'package.json'))).version;
		const beforeNpm = JSON.parse(
			await readFile(join(fixture, 'packages/ditherette/package.json'))
		).version;
		const patchVersion = (version) =>
			version.includes('-rc.')
				? version.split('-')[0]
				: version
						.split('.')
						.map((part, index) => (index === 2 ? String(Number(part) + 1) : part))
						.join('.');
		const run = (program, args) =>
			execFileSync(program, args, { cwd: fixture, encoding: 'utf8', stdio: 'pipe' });
		run('git', ['init', '-b', 'main']);
		run('git', ['add', '.']);
		run('git', [
			'-c',
			'user.name=Release test',
			'-c',
			'user.email=release@example.test',
			'-c',
			'commit.gpgsign=false',
			'commit',
			'-m',
			'fixture'
		]);
		await writeFile(
			join(fixture, '.changeset/fixture.md'),
			`---\n"${target}": patch\n---\n\nFixture release.\n`
		);
		const cli = join(root, 'node_modules/@changesets/cli/bin.js');
		run(process.execPath, [cli, 'status', '--output', join(fixture, 'status.json')]);
		const status = JSON.parse(await readFile(join(fixture, 'status.json')));
		assert.ok(status.releases.some(({ name }) => name === target));
		run(process.execPath, [cli, 'version']);
		await synchronizeReleaseVersion(fixture);
		const web = JSON.parse(await readFile(join(fixture, 'package.json')));
		const npm = JSON.parse(await readFile(join(fixture, 'packages/ditherette/package.json')));
		assert.equal(web.private, true);
		assert.equal(npm.version, target === 'ditherette' ? patchVersion(beforeNpm) : beforeNpm);
		assert.equal(npm.publishConfig.tag, npm.version.includes('-rc.') ? 'rc' : 'latest');
		// A package patch also bumps its website consumer, without tying their version numbers.
		assert.equal(web.version, patchVersion(beforeWeb));
		for (const path of [
			'crates/ditherette-wasm/Cargo.toml',
			'crates/ditherette-wasm/Cargo.lock',
			'crates/ditherette-bench/Cargo.lock'
		]) {
			const source = await readFile(join(fixture, path), 'utf8');
			assert.ok(source.includes(`name = "ditherette-wasm"\nversion = "${npm.version}"`));
		}
		// After a release merge, no remaining changesets must produce an empty status successfully.
		run('git', ['add', '.']);
		run('git', [
			'-c',
			'user.name=Release test',
			'-c',
			'user.email=release@example.test',
			'-c',
			'commit.gpgsign=false',
			'commit',
			'-m',
			'release'
		]);
		run(process.execPath, [cli, 'status', '--output', join(fixture, 'status.json')]);
		assert.deepEqual(JSON.parse(await readFile(join(fixture, 'status.json'))).releases, []);
		const sha = run('git', ['rev-parse', 'HEAD']).trim();
		run('git', ['checkout', '--detach', sha]);
		run('git', ['update-ref', 'refs/remotes/origin/main', sha]);
		run('git', ['branch', '-D', 'main']);
		const repository = 'mia-cx/ditherette';
		const pullRequest = {
			merged_at: '2026-09-24T00:00:00Z',
			merge_commit_sha: sha,
			base: { ref: 'main', repo: { full_name: repository } },
			head: { ref: 'changeset-release/main', repo: { full_name: repository } }
		};
		const plan = execFileSync(process.execPath, [join(root, 'scripts/release-plan.mjs')], {
			cwd: fixture,
			encoding: 'utf8',
			input: JSON.stringify([[pullRequest]]),
			stdio: ['pipe', 'pipe', 'pipe'],
			env: {
				...process.env,
				GITHUB_ACTIONS: 'true',
				GITHUB_SHA: sha,
				GITHUB_REPOSITORY: repository,
				GITHUB_EVENT_NAME: 'push',
				GITHUB_REF: 'refs/heads/main',
				GITHUB_OUTPUT: join(fixture, 'outputs')
			}
		});
		const rc = npm.version.includes('-rc.');
		assert.match(plan, rc ? /^release=false$/m : /^release=true$/m);
		assert.match(plan, rc ? /^web=false$/m : /^web=true$/m);
		assert.match(plan, !rc && target === 'ditherette' ? /^npm=true$/m : /^npm=false$/m);
		assert.match(plan, /^pending=false$/m);
	});
}
