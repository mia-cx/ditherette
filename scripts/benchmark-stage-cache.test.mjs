import assert from 'node:assert/strict';
import test from 'node:test';
import { prepareStageSample, stagePrimeRequest } from './benchmark-stage-cache.mjs';
import { collectCalls } from './benchmark-public-timing.mjs';

test('cross-method prime requests preserve the measured source and relevant settings', () => {
	const source = { width: 1, height: 1, data: new Uint8Array([1, 2, 3, 255]) };
	const output = { width: 2, height: 1, resize: { algorithm: 'area' } };
	const perturb = { field: { algorithm: 'bayer', size: '4' }, strength: 0.7 };
	const quantize = { version: 1, source, palette: [], alpha: {}, matching: 'srgb-euclidean' };
	assert.deepEqual(stagePrimeRequest('process', { source, recipe: { output } }, 'resize'), {
		method: 'resize', request: { version: 1, source, output }
	});
	assert.deepEqual(stagePrimeRequest('separable', { source, dither: { perturb } }, 'perturb'), {
		method: 'perturb', request: { version: 1, source, perturb }
	});
	assert.deepEqual(stagePrimeRequest('quantize', quantize, 'no-dither'), {
		method: 'ditherAndQuantize', request: { ...quantize, dither: { family: 'none' } }
	});
	assert.equal(stagePrimeRequest('resize-lanczos3', { source, output }, 'same-call'), undefined);
	assert.throws(() => stagePrimeRequest('quantize', quantize, 'resize'), /does not match/);
	assert.equal(quantize.dither, undefined);
	assert.deepEqual([...source.data], [1, 2, 3, 255]);
});

test('every warmup and sample gets an observed prime on a fresh instance outside its timer', async () => {
	const events = [];
	let nextId = 0;
	let clock = 0;
	const prepare = () => prepareStageSample({
		create: async () => {
			const id = ++nextId;
			events.push(`create ${id}`);
			return { id, dispose: () => events.push(`close ${id}`) };
		},
		prime: ({ id }) => { events.push(`prime ${id}`); return { id }; },
		observePrime: ({ id }) => events.push(`prime observation ${id}`),
		call: ({ id }) => { events.push(`call ${id}`); return { id }; }
	});
	const result = await collectCalls({
		measurement: { mode: 'single-call', warmup_ms: 1, samples: 3, measurement_ms: 10 },
		prepare,
		observe: ([{ id }]) => events.push(`output observation ${id}`),
		now: () => { events.push('clock'); return clock++; }
	});
	assert.equal(result.warmup_iterations, 1);
	assert.equal(result.sample_ns.length, 3);
	assert.equal(nextId, 4);
	for (let id = 1; id <= nextId; id++) {
		const start = events.indexOf(`create ${id}`);
		assert.deepEqual(events.slice(start, start + 8), [
			`create ${id}`, `prime ${id}`, `prime observation ${id}`, 'clock',
			`call ${id}`, 'clock', `output observation ${id}`, `close ${id}`
		]);
	}
});

test('failed prime or prime observation disposes the new instance and preserves the concrete error', async () => {
	for (const failing of ['prime', 'observe']) {
		const failure = new Error(`${failing} mismatch: actual [9], frozen [1]`);
		let disposed = 0;
		let calls = 0;
		await assert.rejects(prepareStageSample({
			create: async () => ({ dispose: () => disposed++ }),
			prime: () => { if (failing === 'prime') throw failure; return 9; },
			observePrime: () => { throw failure; },
			call: () => calls++
		}), (error) => error === failure);
		assert.equal(disposed, 1);
		assert.equal(calls, 0);
	}
});
