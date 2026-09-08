import assert from 'node:assert/strict';
import test from 'node:test';
import { prepareStageSample, stagePrimeRequest } from './benchmark-stage-cache.mjs';
import { collectCalls } from './benchmark-public-timing.mjs';
import { prepareOperation } from './benchmark-public-page.mjs';
import { events, reset } from './benchmark-stage-cache-fixture.mjs';
import { fileURLToPath } from 'node:url';

test('cross-method prime requests preserve the measured source and relevant settings', () => {
	const source = { width: 1, height: 1, data: new Uint8Array([1, 2, 3, 255]) };
	const output = { width: 2, height: 1, resize: { algorithm: 'area' } };
	const perturb = { field: { algorithm: 'bayer', size: '4' }, strength: 0.7 };
	const quantize = { version: 1, source, palette: [], alpha: {}, matching: 'srgb-euclidean' };
	assert.deepEqual(stagePrimeRequest('process', { source, recipe: { output } }, 'resize'), {
		method: 'resize',
		request: { version: 1, source, output }
	});
	assert.deepEqual(stagePrimeRequest('separable', { source, dither: { perturb } }, 'perturb'), {
		method: 'perturb',
		request: { version: 1, source, perturb }
	});
	assert.deepEqual(stagePrimeRequest('quantize', quantize, 'no-dither'), {
		method: 'ditherAndQuantize',
		request: { ...quantize, dither: { family: 'none' } }
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
	const prepare = () =>
		prepareStageSample({
			create: async () => {
				const id = ++nextId;
				events.push(`create ${id}`);
				return { id, dispose: () => events.push(`close ${id}`) };
			},
			prime: ({ id }) => {
				events.push(`prime ${id}`);
				return { id };
			},
			observePrime: ({ id }) => events.push(`prime observation ${id}`),
			call: ({ id }) => {
				events.push(`call ${id}`);
				return { id };
			}
		});
	const result = await collectCalls({
		measurement: { mode: 'single-call', warmup_ms: 1, samples: 3, measurement_ms: 10 },
		prepare,
		observe: ([{ id }]) => events.push(`output observation ${id}`),
		now: () => {
			events.push('clock');
			return clock++;
		}
	});
	assert.equal(result.warmup_iterations, 1);
	assert.equal(result.sample_ns.length, 3);
	assert.equal(nextId, 4);
	for (let id = 1; id <= nextId; id++) {
		const start = events.indexOf(`create ${id}`);
		assert.deepEqual(events.slice(start, start + 8), [
			`create ${id}`,
			`prime ${id}`,
			`prime observation ${id}`,
			'clock',
			`call ${id}`,
			'clock',
			`output observation ${id}`,
			`close ${id}`
		]);
	}
});

test('failed prime or prime observation disposes the new instance and preserves the concrete error', async () => {
	for (const failing of ['prime', 'observe']) {
		const failure = new Error(`${failing} mismatch: actual [9], frozen [1]`);
		let disposed = 0;
		let calls = 0;
		await assert.rejects(
			prepareStageSample({
				create: async () => ({ dispose: () => disposed++ }),
				prime: () => {
					if (failing === 'prime') throw failure;
					return 9;
				},
				observePrime: () => {
					throw failure;
				},
				call: () => calls++
			}),
			(error) => error === failure
		);
		assert.equal(disposed, 1);
		assert.equal(calls, 0);
	}
});

test('actual adapter checks every declared prime and preserves a transient second-prime mismatch', async () => {
	const previousLocation = Object.getOwnPropertyDescriptor(globalThis, 'location');
	const previousFetch = globalThis.fetch;
	Object.defineProperty(globalThis, 'location', {
		configurable: true,
		value: { href: 'file:///' }
	});
	globalThis.fetch = async () => new Response(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]));
	const quantize = {
		palette: [{ kind: 'color', rgb: [0, 0, 0] }],
		alpha: { mode: 'premultiplied' },
		matching: 'srgb-euclidean'
	};
	const perturb = {
		field: { algorithm: 'bayer', size: '4' },
		space: 'oklab',
		strength: 0.7,
		placement: { mode: 'everywhere' }
	};
	const rgbaReference = {
		dimensions: { width: 1, height: 1 },
		pixels: { format: 'rgba8', data: [1, 2, 3, 255] },
		warnings: []
	};
	const indexedReference = {
		dimensions: { width: 1, height: 1 },
		pixels: {
			format: 'indexed8',
			indices: [0],
			palette_rgba: [0, 0, 0, 255],
			transparent_index: null
		},
		warnings: []
	};
	const variants = [
		[
			'same-call',
			{ operation: 'resize-nearest', anchor: 'center' },
			'resize',
			'resize',
			rgbaReference
		],
		[
			'no-dither',
			{ operation: 'quantize', settings: quantize },
			'ditherAndQuantize',
			'quantize',
			indexedReference
		],
		[
			'perturb',
			{ operation: 'separable', settings: { quantize, perturb } },
			'perturb',
			'ditherAndQuantize',
			rgbaReference
		],
		[
			'resize',
			{
				operation: 'process',
				settings: {
					palette: quantize.palette,
					recipe: {
						version: 1,
						output: { width: 1, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } },
						alpha: quantize.alpha,
						match: quantize.matching,
						dither: { family: 'none' }
					}
				}
			},
			'resize',
			'process',
			rgbaReference
		]
	];
	try {
		for (const [prime, operation, primeMethod, measuredMethod, reference] of variants) {
			const trial = {
				role: 'candidate',
				browser: {
					assets: {
						entries: {
							package: fileURLToPath(
								new URL('./benchmark-stage-cache-fixture.mjs', import.meta.url)
							).slice(1),
							wasm: 'unused.wasm'
						}
					}
				},
				prime_reference_output: reference,
				case: {
					source: { width: 1, height: 1 },
					rgba: [1, 2, 3, 255],
					identity: { output: { width: 1, height: 1 } },
					measurement: { mode: 'single-call', scope: 'complete-call', application_cache: 'warm' },
					browser: {
						operation,
						accepted: 'package',
						candidate: 'package',
						preparation: 'primed-sample',
						cache: {
							roles: { accepted: 'preparation', candidate: 'image-stages', sample_prime: prime }
						}
					}
				}
			};
			reset();
			const prepared = await prepareOperation(trial);
			for (let i = 0; i < 2; i++) {
				const sample = await prepared.prepare();
				try {
					sample.call();
				} finally {
					sample.close();
				}
			}
			prepared.close();
			assert.deepEqual(
				events.filter((e) => e.source).map((e) => [e.type, e.id, e.source]),
				[
					[primeMethod, 2, trial.case.rgba],
					[measuredMethod, 2, trial.case.rgba],
					[primeMethod, 3, trial.case.rgba],
					[measuredMethod, 3, trial.case.rgba]
				]
			);
			reset(3);
			const unstable = await prepareOperation(trial);
			const first = await unstable.prepare();
			try {
				first.call();
			} finally {
				first.close();
			}
			await assert.rejects(unstable.prepare(), /Stage prime differs.*actual/);
			unstable.close();
			assert.equal(
				events.filter((e) => e.type === 'create').length,
				events.filter((e) => e.type === 'dispose').length
			);
			trial.case.browser.preparation = 'primed-instance';
			await assert.rejects(prepareOperation(trial), /Invalid preparation-cache/);
		}
	} finally {
		globalThis.fetch = previousFetch;
		if (previousLocation) Object.defineProperty(globalThis, 'location', previousLocation);
		else delete globalThis.location;
	}
});
