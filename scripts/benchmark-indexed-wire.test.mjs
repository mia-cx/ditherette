import assert from 'node:assert/strict';
import test from 'node:test';
import { constants } from 'node:buffer';
import { cp, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { startAssetServer } from './benchmark-public-browser.mjs';
import { encodeOutput, decodeOutput, usesIndexedWire } from './benchmark-indexed-wire.mjs';

const output = () => ({
	dimensions: { width: 2, height: 1 },
	pixels: {
		format: 'indexed8',
		indices: new Uint8Array([0, 1]),
		palette_rgba: [10, 20, 30, 255, 0, 0, 0, 0],
		transparent_index: 1
	},
	warnings: [{ code: 'transparent-fallback', message: 'fixture' }]
});

test('compact indexed transport preserves every byte and metadata field independently', () => {
	const original = output();
	const encoded = encodeOutput(original);
	assert.equal(encoded.indices_hex, '0001');
	assert.deepEqual(encoded.metadata.pixels.indices, []);
	const decoded = decodeOutput(JSON.parse(JSON.stringify(encoded)), original.dimensions);
	assert.deepEqual(decoded, original);
	original.pixels.indices[0] = 1;
	original.pixels.palette_rgba[0] = 99;
	original.warnings[0].message = 'changed';
	assert.equal(encoded.indices_hex, '0001');
	assert.equal(decoded.pixels.indices[0], 0);
	assert.equal(decoded.pixels.palette_rgba[0], 10);
	assert.equal(decoded.warnings[0].message, 'fixture');
});

test('compact wire rejects malformed lengths, encoding, indices and metadata with bounded errors', () => {
	for (const mutate of [
		(value) => (value.wire_encoding = 'unknown'),
		(value) => (value.indices_hex = '0'),
		(value) => (value.indices_hex = '000000'),
		(value) => (value.indices_hex = '00AA'),
		(value) => (value.indices_hex = '00gg'),
		(value) => (value.indices_hex = '0002'),
		(value) => value.metadata.pixels.indices.push(0),
		(value) => (value.metadata.pixels.palette_rgba[0] = 256),
		(value) => (value.metadata.pixels.transparent_index = 2),
		(value) => (value.metadata.warnings[0].code = 'unknown'),
		(value) => (value.metadata.warnings[0].message = 'x'.repeat(89)),
		(value) => (value.metadata.dimensions.width = 8192 * 8192 + 1),
		(value) => (value.extra = true)
	]) {
		const encoded = encodeOutput(output());
		mutate(encoded);
		assert.throws(
			() => decodeOutput(encoded, { width: 2, height: 1 }),
			(error) => error.message.length < 100
		);
	}
	assert.throws(() => decodeOutput(encodeOutput(output()), { width: 1, height: 2 }), /dimensions/);
	assert.equal(usesIndexedWire({ case: { browser: {} } }), false);
	assert.equal(
		usesIndexedWire({ case: { browser: { retained_output_limit_bytes: 384 * 1024 * 1024 } } }),
		true
	);
});

test('encoding crosses chunk boundaries with canonical byte order', () => {
	const value = output();
	value.dimensions.width = 16 * 1024 + 1;
	value.pixels.indices = Uint8Array.from(
		{ length: value.dimensions.width },
		(_, index) => index % 2
	);
	assert.deepEqual(decodeOutput(encodeOutput(value)), value);
});

test('three capped hex outputs stay below the Node string bound without allocating them', () => {
	const resultBound = 8192 * 8192 * 2 * 3 + 1024 * 1024;
	assert.ok(resultBound < constants.MAX_STRING_LENGTH);
});

test('oracle wrapper compacts only declared cases after the unchanged serializer returns', async () => {
	const root = await mkdtemp(path.join(tmpdir(), 'ditherette-wire-oracle-'));
	const previousFetch = globalThis.fetch;
	try {
		await mkdir(path.join(root, 'oracle'));
		for (const name of ['benchmark-oracle-page.mjs', 'benchmark-indexed-wire.mjs'])
			await cp(new URL(name, import.meta.url), path.join(root, name));
		const original = output();
		original.pixels.indices = Array.from(original.pixels.indices);
		const reference = { case: { fixture: 'independent oracle' }, output: original };
		await writeFile(
			path.join(root, 'oracle/ditherette_bench_oracle.js'),
			`export default async function init() {}\nexport function evaluate() { return ${JSON.stringify(JSON.stringify(reference))}; }\nexport function evaluate_prime() { throw new Error('unexpected prime'); }\n`
		);
		globalThis.fetch = async () => ({ arrayBuffer: async () => new ArrayBuffer(0) });
		const { runTrial } = await import(pathToFileURL(path.join(root, 'benchmark-oracle-page.mjs')));
		const trial = {
			case: {
				source: { width: 1, height: 1 },
				rgba: [0, 0, 0, 255],
				identity: { output: original.dimensions },
				browser: { operation: { operation: 'process' } }
			}
		};
		assert.deepEqual(await runTrial(trial), reference);
		trial.case.browser.retained_output_limit_bytes = 384 * 1024 * 1024;
		const compact = await runTrial(trial);
		assert.deepEqual(compact.case, reference.case);
		assert.deepEqual(decodeOutput(compact.output, original.dimensions), output());
	} finally {
		globalThis.fetch = previousFetch;
		await rm(root, { recursive: true, force: true });
	}
});

test('compact HTTP results enforce their smaller bound while ordinary results keep their existing bound', async () => {
	for (const compact of [true, false]) {
		const trial = {
			case: {
				identity: { output: { width: 10_000, height: 1 } },
				measurement: { samples: 5 },
				browser: compact ? { retained_output_limit_bytes: 384 * 1024 * 1024 } : {}
			}
		};
		const server = await startAssetServer(
			{ tree: { root: process.cwd(), files: [] }, entries: {} },
			false,
			trial
		);
		try {
			const response = await fetch(server.url + server.resultUrl, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ fixture: 'x'.repeat(120_000) })
			});
			assert.equal(response.status, compact ? 413 : 204);
			assert.deepEqual(server.failures, compact ? ['Result body exceeds declared bound.'] : []);
		} finally {
			server.instance.closeAllConnections();
			await new Promise((resolve) => server.instance.close(resolve));
		}
	}
});
