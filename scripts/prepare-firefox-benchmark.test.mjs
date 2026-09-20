import assert from 'node:assert/strict';
import test from 'node:test';
import { enableOptimizedWasm } from './prepare-firefox-benchmark.mjs';

test('older Juggler enables optimized Wasm before attaching debuggees', () => {
	const source = 'this._debugger = new Debugger();\nthis._debugger.addDebuggee(global);';
	const patched = enableOptimizedWasm(source);
	assert.equal(patched, 'this._debugger = new Debugger();\n    this._debugger.allowUnobservedWasm = true;\nthis._debugger.addDebuggee(global);');
	assert.equal(enableOptimizedWasm(patched), patched);
});

test('newer upstream runtime remains untouched', () => {
	const source = 'this._debugger = new Debugger();\nthis._debugger.allowUnobservedWasm = true;\nthis._debugger.allowUnobservedAsmJS = true;';
	assert.equal(enableOptimizedWasm(source), source);
});

test('unknown or ambiguous debugger setup requires inspection', () => {
	assert.throws(() => enableOptimizedWasm('new DifferentDebugger()'), /Unrecognized/);
	assert.throws(() => enableOptimizedWasm('this._debugger = new Debugger();\nthis._debugger = new Debugger();'), /Unrecognized/);
});
