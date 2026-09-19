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
		assert.deepEqual(
			manifest.inputs.map(({ path }) => path),
			[
				'scripts/benchmark-typescript.ts',
				'src/lib/processing/color.ts',
				'src/lib/processing/compositing.ts',
				'src/lib/processing/resize.ts',
				'src/lib/processing/schemas.ts',
				'src/lib/processing/types.ts'
			]
		);
		const adapter = await import(pathToFileURL(path.join(output, manifest.entry)));
		assert.equal(typeof adapter.resize, 'function');
		const resize = await readFile(path.join(output, 'src/lib/processing/resize.js'), 'utf8');
		assert.match(resize, /Math\.round\(sourceX\)/);
		await assert.rejects(prepareTypeScript(output), { code: 'EEXIST' });
	} finally {
		await rm(temporary, { recursive: true, force: true });
	}
});
