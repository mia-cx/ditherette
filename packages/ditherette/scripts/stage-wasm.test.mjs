import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import test from 'node:test';
import ts from 'typescript';
import { stageWasm } from './stage-wasm.mjs';

test('staged declarations resolve source imports and emitted package-relative assets', async (t) => {
	const directory = await mkdtemp(join(tmpdir(), 'ditherette-staging-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const root = pathToFileURL(directory + '/');
	const crate = new URL('crate/', root);
	const packageDirectory = new URL('package/', root);
	await mkdir(new URL('dist/scalar/', crate), { recursive: true });
	await mkdir(new URL('dist/threads/snippets/', crate), { recursive: true });
	await mkdir(new URL('src/', packageDirectory), { recursive: true });
	await writeFile(new URL('LICENSE', root), 'MIT fixture');
	await writeFile(new URL('package.json', packageDirectory), '{"type":"module"}');
	await writeFile(
		new URL('dist/scalar/ditherette_wasm.js', crate),
		'export function hello(name) { return name; }'
	);
	await writeFile(
		new URL('dist/scalar/ditherette_wasm.d.ts', crate),
		'export declare function hello(name: string): string;'
	);
	await writeFile(
		new URL('dist/scalar/ditherette_wasm.factory.js', crate),
		'export function createScalarBindings() { return { hello: (name) => name }; }'
	);
	await writeFile(
		new URL('dist/scalar/ditherette_wasm.factory.d.ts', crate),
		'export declare function createScalarBindings(): typeof import("./ditherette_wasm.js");'
	);
	await writeFile(
		new URL('dist/scalar/ditherette_wasm_bg.wasm', crate),
		new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])
	);
	await writeFile(new URL('dist/threads/snippets/worker.js', crate), '// threaded worker fixture');
	await writeFile(
		new URL('src/index.ts', packageDirectory),
		`
import { createScalarBindings } from './wasm/scalar/ditherette_wasm.factory.js';
export function greet(name: string): string { return createScalarBindings().hello(name); }
`
	);
	await stageWasm(crate, packageDirectory, new URL('LICENSE', root));
	const config = JSON.parse(await readFile(new URL('../tsconfig.json', import.meta.url), 'utf8'));
	const parsed = ts.parseJsonConfigFileContent(config, ts.sys, join(directory, 'package'));
	assert.deepEqual(parsed.errors, []);
	const program = ts.createProgram(parsed.fileNames, parsed.options);
	const diagnostics = ts.getPreEmitDiagnostics(program);
	assert.equal(
		diagnostics.length,
		0,
		ts.formatDiagnosticsWithColorAndContext(diagnostics, {
			getCanonicalFileName: (name) => name,
			getCurrentDirectory: () => directory,
			getNewLine: () => '\n'
		})
	);
	assert.equal(program.emit().emitSkipped, false);
	const emitted = await readFile(new URL('dist/index.js', packageDirectory), 'utf8');
	assert.match(emitted, /\.\/wasm\/scalar\/ditherette_wasm\.factory\.js/);
	const built = await import(new URL('dist/index.js', packageDirectory).href);
	assert.equal(built.greet('typed factory'), 'typed factory');
	assert.equal(
		await readFile(new URL('dist/wasm/threads/snippets/worker.js', packageDirectory), 'utf8'),
		'// threaded worker fixture'
	);
	assert.equal(await readFile(new URL('LICENSE', packageDirectory), 'utf8'), 'MIT fixture');
});

test('staging rejects stale scalar output without factory declarations', async (t) => {
	const directory = await mkdtemp(join(tmpdir(), 'ditherette-staging-missing-'));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const root = pathToFileURL(directory + '/');
	await assert.rejects(
		stageWasm(new URL('crate/', root), new URL('package/', root), new URL('LICENSE', root)),
		/ditherette_wasm\.factory\.js/
	);
});
