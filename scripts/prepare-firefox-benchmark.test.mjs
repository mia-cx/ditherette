import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readlink, rm, symlink, utimes, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { enableOptimizedWasm, prepareFirefoxBenchmark } from './prepare-firefox-benchmark.mjs';

test('older Juggler enables optimized Wasm before attaching debuggees', () => {
	const source = 'class Runtime { constructor() {\nthis._debugger = new Debugger();\nthis._debugger.addDebuggee(global);\n} }';
	const patched = enableOptimizedWasm(source);
	assert.equal(patched, 'class Runtime { constructor() {\nthis._debugger = new Debugger();\n    this._debugger.allowUnobservedWasm = true;\nthis._debugger.addDebuggee(global);\n} }');
	assert.equal(enableOptimizedWasm(patched), patched);
});

test('newer upstream runtime remains untouched', () => {
	const source = 'class Runtime { constructor() {\nthis._debugger = new Debugger();\nthis._debugger.allowUnobservedWasm = true;\nthis._debugger.allowUnobservedAsmJS = true;\n} }';
	assert.equal(enableOptimizedWasm(source), source);
});

test('unknown or ambiguous debugger setup requires inspection', () => {
	assert.throws(() => enableOptimizedWasm('new DifferentDebugger()'), /Unrecognized/);
	assert.throws(() => enableOptimizedWasm('this._debugger = new Debugger();\nthis._debugger = new Debugger();'), /Unrecognized/);
});

test('rejects misleading or inactive already-patched layouts', () => {
	assert.throws(() => enableOptimizedWasm('// this._debugger.allowUnobservedWasm = true;\nthis._debugger = new DifferentDebugger();'), /Unrecognized/);
	assert.throws(() => enableOptimizedWasm('class Runtime { constructor() {\nthis._debugger = new Debugger();\nif (false) this._debugger.allowUnobservedWasm = true;\n} }'), /Unrecognized/);
	assert.throws(() => enableOptimizedWasm('class Runtime { constructor() {\nthis._debugger = new Debugger();\nthis._debugger.addDebuggee(global);\nthis._debugger.allowUnobservedWasm = true;\n} }'), /Unrecognized/);
	assert.throws(() => enableOptimizedWasm('class Runtime { constructor() {\nthis._debugger = new Debugger();\nthis._debugger.allowUnobservedWasm = true;\nthis._debugger = new DifferentDebugger();\n} }'), /Unrecognized/);
});

test('preserves internal links and replaces an older archive entry', async () => {
	const root = await mkdtemp(path.join(os.tmpdir(), 'ditherette-firefox-preparer-'));
	try {
		const original = path.join(root, 'firefox');
		const runtimeEntry = 'chrome/juggler/content/content/Runtime.js';
		const runtime = path.join(original, runtimeEntry);
		await mkdir(path.dirname(runtime), { recursive: true });
		await writeFile(runtime, 'class Runtime { constructor() {\nthis._debugger = new Debugger();\nthis._debugger.addDebuggee(global);\n} }');
		await utimes(runtime, new Date('2099-01-01T00:00:00Z'), new Date('2099-01-01T00:00:00Z'));
		await writeFile(path.join(original, 'firefox-bin'), 'executable');
		await writeFile(path.join(original, 'playwright.cfg'), '// config');
		await mkdir(path.join(original, 'bin'), { recursive: true });
		await writeFile(path.join(original, 'bin', 'payload'), 'payload');
		await symlink('payload', path.join(original, 'bin', 'alias'));
		execFileSync('zip', ['-q', '-r', 'omni.ja', 'chrome'], { cwd: original });

		const prepared = path.join(root, 'prepared');
		await prepareFirefoxBenchmark(path.join(original, 'firefox-bin'), prepared);
		assert.equal(await readlink(path.join(prepared, 'bin', 'alias')), 'payload');
		assert.match(
			execFileSync('unzip', ['-p', path.join(prepared, 'omni.ja'), runtimeEntry], { encoding: 'utf8' }),
			/allowUnobservedWasm = true/,
		);
	} finally {
		await rm(root, { recursive: true, force: true });
	}
});
