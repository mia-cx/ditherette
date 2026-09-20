import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, readdir, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import test from 'node:test';
import ts from 'typescript';
import { prepareTypeScript } from './prepare-benchmark-typescript.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const providerRevision = 'a895267baea624a6e89bfcef6c5147f170e8a8f7';
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');

test('offline compiler emits the historical Git closure without any checked-out source files', async () => {
	const temporary = await mkdtemp(path.join(tmpdir(), 'ditherette-ts-closure-'));
	try {
		const sourceCheckout = path.join(temporary, 'source');
		execFileSync('git', ['clone', '--shared', '--no-checkout', root, sourceCheckout], {
			stdio: 'pipe'
		});
		assert.deepEqual(await readdir(sourceCheckout), ['.git']);
		const output = path.join(temporary, 'compiled');
		const manifest = await prepareTypeScript(output, { sourceCheckout });
		assert.deepEqual(manifest.provider, {
			kind: 'historical-website-typescript',
			source_revision: providerRevision
		});
		assert.equal(
			manifest.source_revision,
			execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim()
		);
		assert.notEqual(manifest.source_revision, manifest.provider.source_revision);
		assert.deepEqual(
			JSON.parse(await readFile(path.join(output, 'compiler-inputs.json'), 'utf8')),
			manifest
		);
		assert.equal(manifest.compiler.version, '6.0.3');
		assert.equal(manifest.compiler.sha256.length, 64);
		const paths = manifest.inputs.map(({ path }) => path);
		for (const required of [
			'scripts/benchmark-typescript.ts',
			'src/lib/processing/resize.ts',
			'src/lib/processing/quantize.ts',
			'src/lib/processing/package-fallback.ts',
			'src/lib/processing/quantize-algorithms/direct.ts'
		])
			assert.ok(paths.includes(required), required);
		assert.equal(new Set(paths).size, paths.length);
		for (const input of manifest.inputs) {
			const source = execFileSync('git', ['show', `${providerRevision}:${input.path}`], {
				cwd: root,
				encoding: 'utf8'
			});
			assert.equal(input.sha256, digest(source), input.path);
			if (input.path !== 'scripts/benchmark-typescript.ts') continue;
			assert.equal(
				input.sha256,
				'0fea5893cbfaf1de96f67e2ba8898651c52411d9e7cd84e6cde9a425911abf04'
			);
			const expected = ts
				.transpileModule(source, {
					fileName: input.path,
					compilerOptions: manifest.options
				})
				.outputText.replace(/from '([^']+)'/g, (_, specifier) => `from "${specifier}.js"`);
			assert.equal(await readFile(path.join(output, manifest.entry), 'utf8'), expected);
		}
		const adapter = await import(pathToFileURL(path.join(output, manifest.entry)));
		assert.equal(typeof adapter.resize, 'function');
		assert.equal(typeof adapter.prepareIndexed, 'function');
		const previousImageData = globalThis.ImageData;
		globalThis.ImageData = class {
			constructor(data, width, height) {
				if (typeof data === 'number') {
					height = width;
					width = data;
					data = new Uint8ClampedArray(width * height * 4);
				}
				Object.assign(this, { data, width, height });
			}
		};
		try {
			const request = {
				version: 1,
				source: {
					width: 3,
					height: 1,
					data: Uint8Array.of(255, 255, 255, 255, 0, 0, 0, 255, 17, 39, 99, 0)
				},
				palette: [
					{ kind: 'color', rgb: [255, 255, 255] },
					{ kind: 'color', rgb: [0, 0, 0] },
					{ kind: 'color', rgb: [255, 255, 255] },
					{ kind: 'transparent' }
				],
				alpha: { mode: 'preserve', threshold: 127 },
				matching: 'srgb-euclidean'
			};
			const call = adapter.prepareIndexed(request);
			const first = call();
			assert.deepEqual([...first.indices], [0, 1, 3]);
			assert.deepEqual(
				[...first.palette.rgba],
				[255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 0]
			);
			assert.equal(first.palette.transparentIndex, 3);
			assert.deepEqual(first.warnings, []);
			const second = call();
			second.indices.fill(42);
			assert.deepEqual([...first.indices], [0, 1, 3]);
			const processRequest = {
				source: request.source,
				palette: request.palette,
				recipe: {
					version: 1,
					output: { width: 6, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } },
					alpha: request.alpha,
					match: request.matching,
					dither: { family: 'none' }
				}
			};
			assert.deepEqual([...adapter.prepareIndexed(processRequest)().indices], [0, 0, 1, 1, 3, 3]);
			assert.throws(
				() => adapter.prepareIndexed({ ...request, matching: 'oklab-euclidean' }),
				/requires/
			);
			assert.throws(
				() =>
					adapter.prepareIndexed({
						...request,
						source: {
							width: 1,
							height: 1,
							data: Uint8Array.of(77, 22, 44, 255)
						}
					}),
				/No faithful/
			);
			assert.throws(
				() =>
					adapter.prepareIndexed({
						...processRequest,
						source: { width: 2, height: 1, data: request.source.data.slice(0, 8) },
						recipe: {
							...processRequest.recipe,
							output: { ...processRequest.recipe.output, width: 49 }
						}
					}),
				/No faithful/
			);
			assert.throws(
				() => adapter.prepareIndexed({ ...request, palette: request.palette.slice(0, 3) }),
				/explicit transparent/
			);
		} finally {
			if (previousImageData === undefined) delete globalThis.ImageData;
			else globalThis.ImageData = previousImageData;
		}
		const resize = await readFile(path.join(output, 'src/lib/processing/resize.js'), 'utf8');
		assert.match(resize, /Math\.round\(sourceX\)/);
		await assert.rejects(prepareTypeScript(output), { code: 'EEXIST' });
	} finally {
		await rm(temporary, { recursive: true, force: true });
	}
});

test('missing historical Git objects fail before emitting a provider', async (t) => {
	const temporary = await mkdtemp(path.join(tmpdir(), 'ditherette-ts-missing-history-'));
	t.after(() => rm(temporary, { recursive: true, force: true }));
	execFileSync('git', ['init', '-q', temporary], { stdio: 'pipe' });
	const output = path.join(temporary, 'compiled');
	await assert.rejects(prepareTypeScript(output, { sourceCheckout: temporary }), (error) => {
		assert.match(error.message, /Historical TypeScript provider commit .* is unavailable/);
		assert.ok(error.message.includes(providerRevision));
		assert.match(error.message, /Fetch its Git objects before offline preparation/);
		assert.ok(error.cause);
		return true;
	});
	await assert.rejects(stat(output), { code: 'ENOENT' });
});
