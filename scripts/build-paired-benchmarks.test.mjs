import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { normalizedArguments } from './build-paired-benchmarks.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const builder = join(root, 'scripts/build-paired-benchmarks.mjs');

function run(program, args, options) {
	const result = spawnSync(program, args, { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024, ...options });
	assert.equal(result.error, undefined);
	assert.equal(result.status, 0, result.stderr);
	return result.stdout.trim();
}

test('compiler recipes bind effective root and dependency profiles across fresh worktrees', (t) => {
	const directory = mkdtempSync(join(tmpdir(), 'ditherette-paired-build-'));
	if (process.env.DITHERETTE_KEEP_BUILD_FIXTURE !== '1') t.after(() => rmSync(directory, { recursive: true }));
	const worktree = join(directory, 'worktree');
	const crate = join(worktree, 'crates/ditherette-bench');
	const subject = join(worktree, 'crates/subject');
	mkdirSync(join(crate, 'src'), { recursive: true });
	mkdirSync(join(subject, 'src'), { recursive: true });
	writeFileSync(join(crate, 'Cargo.toml'), '[package]\nname="ditherette-bench"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nsubject={path="../subject"}\n');
	writeFileSync(join(subject, 'Cargo.toml'), '[package]\nname="subject"\nversion="0.1.0"\nedition="2021"\n');
	writeFileSync(join(subject, 'src/lib.rs'), 'pub fn value() -> u8 { 7 }\n');
	writeFileSync(join(crate, 'src/main.rs'), 'fn main() { assert_eq!(subject::value(), 7); println!("{}\\n{}", option_env!("DITHERETTE_BENCH_RECORDED_BUILD").unwrap_or("false"), env!("DITHERETTE_BENCH_CONFIGURATION")); }\n');
	writeFileSync(join(crate, 'build.rs'), readFileSync(join(root, 'crates/ditherette-bench/build.rs')));
	writeFileSync(join(crate, 'Cargo.lock'), 'version = 4\n[[package]]\nname="ditherette-bench"\nversion="0.1.0"\ndependencies=["subject"]\n[[package]]\nname="subject"\nversion="0.1.0"\n');
	const environment = { ...process.env, CARGO_INCREMENTAL: '0' };
	const manual = [];
	for (const units of [1, 16]) {
		const target = join(directory, `manual-${units}`);
		run('cargo', ['+1.97.0', 'build', '--locked', '--release', '--manifest-path', join(crate, 'Cargo.toml'), '--target-dir', target, '--config', `profile.release.codegen-units=${units}`], { cwd: directory, env: environment });
		manual.push(run(join(target, 'release/ditherette-bench'), [], { env: environment }));
	}
	assert.equal(manual[0], manual[1]);
	assert.ok(manual.every((identity) => identity.startsWith('false\n')));
	const recorded = (name, source, overrides) => {
		const output = run(process.execPath, [builder, source, join(directory, name), ...overrides.flatMap((value) => ['--config', value])], { env: environment });
		const executable = output.split('\n').find((line) => line.startsWith('ditherette-bench\t'))?.split('\t')[1];
		assert.ok(executable);
		const [attested, configuration] = run(executable, [], { env: environment }).split('\n');
		assert.equal(attested, 'true');
		const recipe = JSON.parse(configuration);
		assert.equal(recipe.schema, 'ditherette-rustc-recipe-v1');
		assert.ok(recipe.commands.some((record) => JSON.parse(record).package === 'subject'));
		return configuration;
	};
	const one = recorded('recorded-one', worktree, ['profile.release.codegen-units=1']);
	const sixteen = recorded('recorded-sixteen', worktree, ['profile.release.codegen-units=16']);
	assert.notEqual(one, sixteen);
	const dependency = recorded('recorded-dependency', worktree, ['profile.release.codegen-units=1', 'profile.release.package.subject.codegen-units=16']);
	assert.notEqual(one, dependency);
	const twin = join(directory, 'twin');
	cpSync(worktree, twin, { recursive: true });
	assert.equal(one, recorded('recorded-twin', twin, ['profile.release.codegen-units=1']));
	process.stdout.write(`Compiler recipe evidence: ${directory}\n`);
});

test('host-dependent CPU selection cannot claim a portable recorded recipe', () => {
	for (const args of [
		['-C', 'target-cpu=native'],
		['-Ctarget-cpu=native'],
		['--codegen', 'target-cpu=native'],
		['--codegen=target-cpu=native']
	]) assert.throws(() => normalizedArguments(args, {}), /explicit CPU/);
	const explicit = ['-C', 'target-cpu=x86-64', '-C', 'codegen-units=1'];
	assert.deepEqual(normalizedArguments(explicit, {}), explicit);
});
