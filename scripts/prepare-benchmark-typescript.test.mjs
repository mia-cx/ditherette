import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import test from 'node:test';
import { prepareTypeScript } from './prepare-benchmark-typescript.mjs';

test('offline compiler captures actual local closure and emits loadable browser modules', async () => {
	const temporary = await mkdtemp(path.join(tmpdir(), 'ditherette-ts-closure-'));
	try {
		const output = path.join(temporary, 'compiled');
		const manifest = await prepareTypeScript(output);
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
