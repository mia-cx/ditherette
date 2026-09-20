import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const runtimeEntry = 'chrome/juggler/content/content/Runtime.js';
const allowWasm = 'this._debugger.allowUnobservedWasm = true;';

function tokens(source) {
	const result = [];
	let index = 0;
	while (index < source.length) {
		if (/\s/.test(source[index])) {
			index += 1;
			continue;
		}
		if (source.startsWith('//', index)) {
			const end = source.indexOf('\n', index + 2);
			index = end === -1 ? source.length : end + 1;
			continue;
		}
		if (source.startsWith('/*', index)) {
			const end = source.indexOf('*/', index + 2);
			if (end === -1) return null;
			index = end + 2;
			continue;
		}
		if (source[index] === '"' || source[index] === "'" || source[index] === '`') {
			const quote = source[index++];
			while (index < source.length) {
				if (source[index] === '\\') {
					index += 2;
					continue;
				}
				if (source[index++] === quote) break;
			}
			if (source[index - 1] !== quote) return null;
			continue;
		}
		const identifier = /^[A-Za-z_$][\w$]*/.exec(source.slice(index));
		if (identifier) {
			result.push({ value: identifier[0], start: index, end: index + identifier[0].length });
			index += identifier[0].length;
			continue;
		}
		result.push({ value: source[index], start: index, end: index + 1 });
		index += 1;
	}
	return result;
}

function matches(values, index, expected) {
	return expected.every((value, offset) => values[index + offset] === value);
}

function debuggerLayout(source) {
	const scanned = tokens(source);
	if (!scanned) return null;
	const values = scanned.map(({ value }) => value);
	let constructorEnd;
	let constructorIndex;
	let constructorCount = 0;
	let allowIndex;
	let firstAddDebuggeeIndex;
	for (let index = 0; index < values.length; index += 1) {
		if (matches(values, index, ['this', '.', '_debugger', '='])) {
			if (!matches(values, index + 4, ['new', 'Debugger', '(', ')', ';'])) return null;
			constructorCount += 1;
			constructorIndex = index;
			constructorEnd = scanned[index + 8].end;
		}
		if (matches(values, index, ['this', '.', '_debugger', '.', 'allowUnobservedWasm', '=', 'true', ';'])) {
			if (allowIndex !== undefined) return null;
			allowIndex = index;
		}
		if (matches(values, index, ['this', '.', '_debugger', '.', 'allowUnobservedWasm', '=']) &&
			!matches(values, index, ['this', '.', '_debugger', '.', 'allowUnobservedWasm', '=', 'true', ';'])) return null;
		if (firstAddDebuggeeIndex === undefined &&
			matches(values, index, ['this', '.', '_debugger', '.', 'addDebuggee'])) firstAddDebuggeeIndex = index;
	}
	if (constructorCount !== 1 || (allowIndex !== undefined && allowIndex <= constructorIndex) ||
		(firstAddDebuggeeIndex !== undefined && firstAddDebuggeeIndex <= constructorIndex) ||
		(firstAddDebuggeeIndex !== undefined && allowIndex !== undefined && firstAddDebuggeeIndex < allowIndex)) return null;
	return { constructorEnd, patched: allowIndex !== undefined };
}

/** Keep Juggler automation while allowing Firefox to optimize the measured Wasm. */
export function enableOptimizedWasm(source) {
	const layout = debuggerLayout(source);
	if (!layout) {
		throw new Error('Unrecognized Juggler runtime; inspect its Debugger setup before benchmarking.');
	}
	if (layout.patched) return source;
	return `${source.slice(0, layout.constructorEnd)}\n    ${allowWasm}${source.slice(layout.constructorEnd)}`;
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
	await cp(original, target, { recursive: true, verbatimSymlinks: true });
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
		execFileSync('zip', ['-q', archive, runtimeEntry], { cwd: scratch });
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
	// Juggler's test configuration overrides normal policy discovery. Resolve from
	// the executable directory so paired snapshots can relocate the prepared tree.
	const configPath = path.join(target, 'playwright.cfg');
	const config = await readFile(configPath, 'utf8');
	await writeFile(configPath, `${config}\n// Pin the benchmark runtime even after relocation.\n` +
		`var benchmarkPolicy = Components.classes["@mozilla.org/file/directory_service;1"]\n` +
		`  .getService(Components.interfaces.nsIProperties).get("GreD", Components.interfaces.nsIFile);\n` +
		`benchmarkPolicy.append("distribution");\nbenchmarkPolicy.append("policies.json");\n` +
		`lockPref("browser.policies.alternatePath", benchmarkPolicy.path);\n`);
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
