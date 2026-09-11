import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const runtimeEntry = 'chrome/juggler/content/content/Runtime.js';
const constructor = 'this._debugger = new Debugger();';
const allowWasm = 'this._debugger.allowUnobservedWasm = true;';

/** Keep Juggler automation while allowing Firefox to optimize the measured Wasm. */
export function enableOptimizedWasm(source) {
	if (source.includes(allowWasm)) return source;
	if (source.split(constructor).length !== 2) {
		throw new Error('Unrecognized Juggler runtime; inspect its Debugger setup before benchmarking.');
	}
	return source.replace(constructor, `${constructor}\n    ${allowWasm}`);
}

/** Prepare a separate Firefox tree before snapshotting it for a paired trial. Requires unzip/zip. */
export async function prepareFirefoxBenchmark(executable, destination) {
	if (process.env.DITHERETTE_BENCH_QUIET === '1') {
		throw new Error('Prepare the Firefox runtime before the benchmark quiet phase.');
	}
	const original = path.dirname(path.resolve(executable));
	const target = path.resolve(destination);
	if (target === original || target.startsWith(original + path.sep)) {
		throw new Error('Use a separate new directory, outside the installed Firefox tree.');
	}
	await mkdir(path.dirname(target), { recursive: true });
	await mkdir(target);
	await cp(original, target, { recursive: true });
	const archive = path.join(target, 'omni.ja');
	const hash = async () => createHash('sha256').update(await readFile(archive)).digest('hex');
	const before = await hash();
	const source = execFileSync('unzip', ['-p', archive, runtimeEntry], { encoding: 'utf8' });
	const patched = enableOptimizedWasm(source);
	const scratch = path.join(target, '.benchmark-patch');
	if (patched !== source) {
		const filename = path.join(scratch, runtimeEntry);
		await mkdir(path.dirname(filename), { recursive: true });
		await writeFile(filename, patched);
		execFileSync('zip', ['-q', '-u', archive, runtimeEntry], { cwd: scratch });
		await rm(scratch, { recursive: true });
	}
	const actual = execFileSync('unzip', ['-p', archive, runtimeEntry], { encoding: 'utf8' });
	if (actual !== patched) throw new Error('Firefox runtime patch verification failed.');
	const policyPath = path.join(target, 'distribution/policies.json');
	let policy = {};
	try { policy = JSON.parse(await readFile(policyPath, 'utf8')); }
	catch (error) { if (error.code !== 'ENOENT') throw error; }
	await mkdir(path.dirname(policyPath), { recursive: true });
	await writeFile(policyPath, JSON.stringify({ ...policy, policies: { ...policy.policies, DisableAppUpdate: true } }));
	const result = { executable: path.join(target, path.basename(executable)), source: path.resolve(executable),
		archiveBefore: before, archiveAfter: await hash(), allowUnobservedWasm: true };
	await writeFile(path.join(target, 'benchmark-runtime.json'), JSON.stringify(result, null, 2));
	return result;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
	const [executable, destination, ...extra] = process.argv.slice(2);
	if (!executable || !destination || extra.length) throw new Error('Usage: prepare-firefox-benchmark.mjs FIREFOX_EXECUTABLE NEW_DIRECTORY');
	console.log(JSON.stringify(await prepareFirefoxBenchmark(executable, destination)));
}
