import assert from 'node:assert/strict';
import test from 'node:test';
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
