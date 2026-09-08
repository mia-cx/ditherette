import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import test from 'node:test';
import { scalarFactory, writeScalarFactory } from './scalar-factory.mjs';

const glue = `
import { copy, helperIdentity } from './helper.mjs';
globalThis.__ditheretteFactoryExecutions += 1;
let wasm;
let cachedView;
let references = [];
const asset = new URL('ditherette_wasm_bg.wasm', import.meta.url);
function imports() { return { read: () => wasm.value }; }
function initSync(module) {
  if (wasm !== undefined) return wasm;
  wasm = module;
  cachedView = new Uint8Array(module.memory.buffer);
  return wasm;
}
async function __wbg_init(input) { return initSync(await input.module_or_path); }
export function getImports() { return imports(); }
export function write(value) { copy(cachedView, value); references.push(value); }
export function state() { return [cachedView[0], references.length, asset.pathname]; }
export function sharedHelper() { return helperIdentity; }
export class Reader { read() { return wasm.value; } }
export { initSync, __wbg_init as default };
`;

test('factory imports are inert and each invocation isolates mutable glue and import closures', async (t) => {
	globalThis.__ditheretteFactoryExecutions = 0;
	t.after(() => {
		delete globalThis.__ditheretteFactoryExecutions;
	});
	const directory = await mkdtemp(join(tmpdir(), 'ditherette-factory-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	await writeFile(
		join(directory, 'helper.mjs'),
		`
export const helperIdentity = Object.freeze({});
export function copy(view, value) { view[0] = value; }
`
	);
	const factoryPath = join(directory, 'factory.mjs');
	await writeFile(factoryPath, scalarFactory(glue));
	const { createScalarBindings } = await import(pathToFileURL(factoryPath).href);
	assert.equal(globalThis.__ditheretteFactoryExecutions, 0);
	const first = createScalarBindings();
	const second = createScalarBindings();
	assert.equal(globalThis.__ditheretteFactoryExecutions, 2);
	assert.notEqual(first, second);
	assert.equal(first.sharedHelper(), second.sharedHelper());
	assert.notEqual(first.getImports(), second.getImports());
	const firstModule = { value: 'first', memory: new WebAssembly.Memory({ initial: 1 }) };
	const secondModule = { value: 'second', memory: new WebAssembly.Memory({ initial: 1 }) };
	await Promise.all([
		first.default({ module_or_path: Promise.resolve(firstModule) }),
		second.default({ module_or_path: Promise.resolve(secondModule) })
	]);
	first.write(7);
	second.write(9);
	second.write(11);
	assert.deepEqual(first.state().slice(0, 2), [7, 1]);
	assert.deepEqual(second.state().slice(0, 2), [11, 2]);
	assert.equal(first.getImports().read(), 'first');
	assert.equal(second.getImports().read(), 'second');
	assert.equal(new first.Reader().read(), 'first');
	assert.equal(new second.Reader().read(), 'second');
	assert.equal(first.initSync(secondModule), firstModule);
	assert.equal(first.state()[2], join(directory, 'ditherette_wasm_bg.wasm'));
});

test('generator fails visibly for unsupported exports and changed initializer names', () => {
	for (const extra of [
		'export const count = 1;',
		'export default function replacement() {}',
		'export * from "./other.js";',
		'export { value } from "./other.js";',
		'export { missing };',
		'export { initSync };'
	])
		assert.throws(() => scalarFactory(glue + extra), /Unsupported generated scalar export shape/);
	assert.throws(
		() => scalarFactory(glue.replace('__wbg_init as default', 'initSync as default')),
		/initialization exports changed/
	);
	assert.throws(() => scalarFactory('export function {'), /Cannot parse/);
});

test('generation keeps normal glue intact and emits sibling factory declarations', async (t) => {
	const directory = await mkdtemp(join(tmpdir(), 'ditherette-factory-files-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const url = pathToFileURL(directory + '/');
	await writeFile(new URL('ditherette_wasm.js', url), glue);
	await writeScalarFactory(url);
	assert.equal(await readFile(new URL('ditherette_wasm.js', url), 'utf8'), glue);
	assert.match(
		await readFile(new URL('ditherette_wasm.factory.js', url), 'utf8'),
		/export function createScalarBindings/
	);
	assert.match(
		await readFile(new URL('ditherette_wasm.factory.d.ts', url), 'utf8'),
		/typeof import\("\.\/ditherette_wasm\.js"\)/
	);
});

test('threaded factory replaces only the pinned worker import with instance-owned startup', async () => {
	const threaded = `import { startWorkers as start } from './snippets/wasm-bindgen-rayon-id/src/workerHelpers.no-bundler.js';
function initSync() {}
async function __wbg_init() {}
export function initThreadPool(...args) { return start(...args); }
export class wbg_rayon_PoolBuilder {
  constructor() { this.registered = true; }
  __destroy_into_raw() { this.registered = false; return 123; }
  free() { throw new Error('attempted to take ownership of Rust value while it was borrowed'); }
}
export { initSync, __wbg_init as default };`;
	const output = scalarFactory(threaded, undefined, true);
	assert.doesNotMatch(output, /workerHelpers/);
	const { createThreadedBindings } = await import(`data:text/javascript,${encodeURIComponent(output)}`);
	const calls = [];
	const first = createThreadedBindings(async (...args) => calls.push(['first', ...args]));
	const second = createThreadedBindings(async (...args) => calls.push(['second', ...args]));
	await first.initThreadPool(1);
	await second.initThreadPool(2);
	assert.deepEqual(calls, [['first', 1], ['second', 2]]);
	const borrowed = new first.wbg_rayon_PoolBuilder();
	first.abandonThreadPool(borrowed);
	assert.equal(borrowed.registered, false, 'Abandonment unregisters without consuming borrowed Rust.');
	assert.throws(() => scalarFactory(glue, undefined, true), /worker import changed/);
	assert.throws(() => scalarFactory(threaded.replace('startWorkers as start', 'other as start'), undefined, true), /Unsupported/);
});
