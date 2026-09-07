import assert from 'node:assert/strict';
import test from 'node:test';
import { DitheretteError } from '../dist/errors.js';
import { validateOptions, validateResize } from '../dist/validation.js';

const request = () => ({
	version: 1,
	source: { width: 1, height: 1, data: new Uint8Array([17, 31, 47, 127]) },
	output: { width: 2, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } }
});
const fails = (run, code, path) =>
	assert.throws(
		run,
		(error) => error instanceof DitheretteError && error.code === code && error.path === path
	);

test('area and bilinear accept only their frozen fields', () => {
	const area = request();
	area.output.resize = { algorithm: 'area' };
	assert.equal(validateResize(area).algorithm, 1);
	area.output.resize.anchor = 'center';
	fails(() => validateResize(area), 'invalid-settings', 'output.resize.anchor');
	const bilinear = request();
	bilinear.output.resize.algorithm = 'bilinear';
	assert.equal(validateResize(bilinear).algorithm, 2);
	bilinear.output.resize.support = 'fixed';
	fails(() => validateResize(bilinear), 'invalid-settings', 'output.resize.support');
});

test('trilinear uses tag six, all anchors, and no support field', () => {
	for (const [anchor, name] of [
		'top-left',
		'top',
		'top-right',
		'left',
		'center',
		'right',
		'bottom-left',
		'bottom',
		'bottom-right'
	].entries()) {
		const value = request();
		value.output.resize = { algorithm: 'trilinear', anchor: name };
		const result = validateResize(value);
		assert.equal(result.algorithm, 6);
		assert.equal(result.anchor, anchor);
		assert.equal(result.support, 0);
		value.output.resize.support = 'fixed';
		fails(() => validateResize(value), 'invalid-settings', 'output.resize.support');
	}
});

test('convolution requires canonical support and reads each setting once', () => {
	for (const [offset, algorithm] of ['bicubic', 'lanczos2', 'lanczos3'].entries()) {
		for (const [support, name] of ['fixed', 'scale-aware'].entries()) {
			const value = request();
			let reads = 0;
			value.output.resize = {
				algorithm,
				anchor: 'bottom-right',
				get support() {
					reads++;
					return name;
				}
			};
			const validated = validateResize(value);
			assert.equal(validated.algorithm, offset + 3);
			assert.equal(validated.support, support);
			assert.equal(validated.anchor, 8);
			assert.equal(reads, 1);
		}
		for (const support of [undefined, null, 0, true, 'auto', ['fixed'], { fixed: null }]) {
			const value = request();
			value.output.resize = { algorithm, anchor: 'center', support };
			fails(() => validateResize(value), 'invalid-settings', 'output.resize.support');
		}
		const missing = request();
		missing.output.resize = { algorithm, anchor: 'center' };
		fails(() => validateResize(missing), 'invalid-settings', 'output.resize.support');
	}
});

test('nearest validation preserves byte views and the frozen anchor order', () => {
	const anchors = [
		'top-left',
		'top',
		'top-right',
		'left',
		'center',
		'right',
		'bottom-left',
		'bottom',
		'bottom-right'
	];
	for (const [anchor, name] of anchors.entries()) {
		const value = request();
		value.output.resize.anchor = name;
		const padded = new Uint8Array([9, ...value.source.data, 9]);
		value.source.data = padded.subarray(1, 5);
		assert.deepEqual(validateResize(value), {
			data: value.source.data,
			sourceWidth: 1,
			sourceHeight: 1,
			outputWidth: 2,
			outputHeight: 1,
			algorithm: 0,
			anchor,
			support: 0
		});
		assert.equal(validateResize(value).data.buffer, value.source.data.buffer);
		assert.equal(validateResize(value).data.byteOffset, 1);
	}
});

test('canonical raw request shapes reject coercions, sequence tags, and extra fields', () => {
	for (const value of [null, [], 'request', 1, undefined])
		fails(() => validateResize(value), 'invalid-request', 'request');
	const cases = [
		[(r) => (r.version = '1'), 'invalid-request', 'version'],
		[(r) => (r.version = 2), 'invalid-request', 'version'],
		[(r) => (r.extra = true), 'invalid-request', 'request.extra'],
		[(r) => (r.source.extra = true), 'invalid-image', 'source.extra'],
		[(r) => (r.output.extra = true), 'invalid-settings', 'output.extra'],
		[(r) => (r.output.resize = ['nearest']), 'invalid-settings', 'output.resize'],
		[
			(r) => (r.output.resize.algorithm = { nearest: null }),
			'invalid-settings',
			'output.resize.algorithm'
		],
		[
			(r) => (r.output.resize.anchor = { center: null }),
			'invalid-settings',
			'output.resize.anchor'
		],
		[(r) => (r.output.resize.anchor = ['center']), 'invalid-settings', 'output.resize.anchor'],
		[(r) => (r.output.resize.anchor = 'bogus'), 'invalid-settings', 'output.resize.anchor'],
		[(r) => (r.output.resize.support = 'fixed'), 'invalid-settings', 'output.resize.support'],
		[(r) => (r.output.resize.other = false), 'invalid-settings', 'output.resize.other'],
		[(r) => (r.output.resize.algorithm = 'bogus'), 'invalid-settings', 'output.resize.algorithm'],
		[(r) => (r.onProgress = () => {}), 'unsupported-operation', 'onProgress'],
		[(r) => (r.onProgress = false), 'invalid-settings', 'onProgress'],
		[(r) => (r.source.data = [17, 31, 47, 127]), 'invalid-image', 'source.data'],
		[(r) => (r.source.data = new Uint8ClampedArray(4)), 'invalid-image', 'source.data'],
		[(r) => (r.source.data = new Uint8Array(3)), 'invalid-image', 'source.data']
	];
	for (const [mutate, code, path] of cases) {
		const value = request();
		mutate(value);
		fails(() => validateResize(value), code, path);
	}
	const detached = request();
	structuredClone(detached.source.data.buffer, { transfer: [detached.source.data.buffer] });
	fails(() => validateResize(detached), 'invalid-image', 'source.data');
	const overridden = request();
	Object.defineProperty(overridden.source.data, 'length', {
		get() {
			throw new Error('caller override');
		}
	});
	Object.defineProperty(overridden.source.data, 'byteLength', {
		get() {
			throw new Error('caller override');
		}
	});
	assert.equal(validateResize(overridden).data.length, 4);
});

test('dimension validation enforces integer side and pixel limits before reading data', () => {
	for (const width of [0, -1, 1.5, NaN, Infinity, '1', 32_769]) {
		const value = request();
		value.source.width = width;
		fails(() => validateResize(value), 'invalid-image', 'source.width');
	}
	for (const width of [0, -1, 1.5, NaN, Infinity, '1', 16_385]) {
		const value = request();
		value.output.width = width;
		fails(() => validateResize(value), 'invalid-settings', 'output.width');
	}
	const sourcePixels = request();
	sourcePixels.source.width = 32_768;
	sourcePixels.source.height = 2_049;
	Object.defineProperty(sourcePixels.source, 'data', {
		get() {
			throw new Error('must not read');
		}
	});
	fails(() => validateResize(sourcePixels), 'invalid-image', 'source');
	const outputPixels = request();
	outputPixels.output.width = 16_384;
	outputPixels.output.height = 4_097;
	fails(() => validateResize(outputPixels), 'invalid-settings', 'output');
});

test('options retain custom inputs and enforce explicit scalar/memory settings', () => {
	assert.deepEqual(validateOptions(undefined), {
		threads: 'disabled',
		memoryLimitBytes: 1_610_612_736,
		wasm: undefined
	});
	for (const memoryLimitBytes of [1, 2_147_483_648])
		assert.equal(validateOptions({ memoryLimitBytes }).memoryLimitBytes, memoryLimitBytes);
	for (const memoryLimitBytes of [0, -1, 1.5, NaN, Infinity, '8', 2_147_483_649])
		fails(() => validateOptions({ memoryLimitBytes }), 'invalid-settings', 'memoryLimitBytes');
	for (const threads of [null, true, 'auto', ['disabled'], { disabled: null }])
		fails(() => validateOptions({ threads }), 'invalid-settings', 'threads');
	for (const threads of ['disabled', 'preferred', 'required'])
		assert.equal(validateOptions({ threads }).threads, threads);
	for (const wasm of [
		'module.wasm',
		new URL('https://example.test/module.wasm'),
		new Response(),
		new ArrayBuffer(8),
		new Uint8Array(8).subarray(2),
		new WebAssembly.Module(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]))
	]) {
		assert.equal(validateOptions({ wasm }).wasm, wasm);
	}
	for (const wasm of [null, 1, {}, [], Promise.resolve(new ArrayBuffer(8))])
		fails(() => validateOptions({ wasm }), 'invalid-settings', 'wasm');
	fails(() => validateOptions({ backend: 'scalar' }), 'invalid-settings', 'backend');
});

test('request property failures are structured and fields are read once', () => {
	const value = request();
	let reads = 0;
	Object.defineProperty(value.source, 'width', {
		get() {
			reads += 1;
			return 1;
		}
	});
	validateResize(value);
	assert.equal(reads, 1);
	Object.defineProperty(value, 'version', {
		get() {
			throw new Error('getter failed');
		}
	});
	fails(() => validateResize(value), 'invalid-request', 'request');
});
