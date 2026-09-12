import { spawnSync } from 'node:child_process';
import { copyFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const result = spawnSync('cargo', [
	'build', '--locked', '--release', '--manifest-path', 'crates/ditherette-bench/Cargo.toml',
	'--bins', '--bench', 'crit_spec_nearest', '--message-format=json-render-diagnostics'
], { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'], maxBuffer: 16 * 1024 * 1024 });
if (result.error) throw result.error;
if (result.status !== 0) process.exit(result.status ?? 1);
const targets = new Map(['ditherette-bench', 'ditherette-bench-lease', 'crit_spec_nearest'].map((name) => [name, null]));
for (const line of result.stdout.split('\n').filter(Boolean)) {
	const artifact = JSON.parse(line);
	if (artifact.reason === 'compiler-artifact' && artifact.executable && targets.has(artifact.target.name)) {
		targets.set(artifact.target.name, artifact.executable);
	}
}
for (const [name, executable] of targets) {
	if (!executable) throw new Error(`Cargo did not produce ${name}`);
}
const prepared = new URL('crates/ditherette-bench/target/prepared/', root);
await mkdir(prepared, { recursive: true });
for (const [name, executable] of targets) {
	await copyFile(executable, new URL(name, prepared));
}
console.log(`Prepared benchmark executables in ${fileURLToPath(prepared)}. Drain agents, builds and tests before invoking bench:*.`);
