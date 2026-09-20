import assert from 'node:assert/strict';
import test from 'node:test';
import {
	collectCalls,
	collectInitializations,
	timeCalls,
	timeInitialization,
	retainedOutputSlots,
	RETAINED_OUTPUT_LIMIT,
	RESULT_BOOKKEEPING_BYTES,
	STABILITY_EVIDENCE_SLOTS
} from './benchmark-public-timing.mjs';

test('an explicit capped-output budget admits one max-area indexed result without changing the default', () => {
	const outputBytes = 8192 * 8192 + 1024;
	const limit = 384 * 1024 * 1024;
	assert.throws(() => retainedOutputSlots(1, outputBytes), /64 MiB/);
	assert.equal(retainedOutputSlots(1, outputBytes, limit).length, 1);
	assert.throws(() => retainedOutputSlots(2, outputBytes, limit), /stability budget/);
	const required = (1 + STABILITY_EVIDENCE_SLOTS) * (outputBytes + RESULT_BOOKKEEPING_BYTES);
	assert.equal(retainedOutputSlots(1, outputBytes, required).length, 1);
	assert.throws(() => retainedOutputSlots(1, outputBytes, required - 1), /stability budget/);
	for (const invalid of [0, -1, 1.5, NaN, Infinity, null, '402653184', limit + 1])
		assert.throws(() => retainedOutputSlots(1, 0, invalid), /retained output limit/);
});

test('explicit retention reaches both collectors without adding observations to their timers', async () => {
	for (const initialization of [false, true]) {
		let clock = 0;
		let calls = 0;
		let closes = 0;
		let observations = 0;
		const measurement = { mode: 'single-call', samples: 5, warmup_ms: 1, measurement_ms: 100 };
		const options = {
			measurement,
			outputBytes: 8192 * 8192 + 1024, // Declared bytes only; the mock allocates no image.
			retainedOutputLimit: 384 * 1024 * 1024,
			now: () => clock,
			observe(outputs) {
				assert.equal(outputs.length, 1);
				assert.equal(outputs[0], calls);
				observations++;
				clock += 100;
			}
		};
		const result = initialization
			? await collectInitializations({
					...options,
					create: async () => {
						clock++;
						return { dispose: () => closes++ };
					},
					probe: () => ++calls
				})
			: await collectCalls({
					...options,
					prepare: async () => ({
						call: () => {
							clock++;
							return ++calls;
						},
						close: () => closes++
					})
				});
		assert.deepEqual(result.sample_ns, Array(5).fill(1e6));
		assert.deepEqual([calls, observations, closes], [6, 6, 6]);
	}
});

test('retained output budget includes evidence and fails before oversized samples without reducing their count', async () => {
	const bytes =
		Math.floor(RETAINED_OUTPUT_LIMIT / (2 + STABILITY_EVIDENCE_SLOTS)) - RESULT_BOOKKEEPING_BYTES;
	assert.equal(retainedOutputSlots(2, bytes).length, 2);
	assert.throws(() => retainedOutputSlots(2, bytes + 1), /64 MiB/);
	let calls = 0,
		prepares = 0,
		closes = 0,
		clock = 0;
	await assert.rejects(
		collectCalls({
			measurement: { mode: 'throughput', samples: 5, warmup_ms: 1, target_sample_ms: 100 },
			outputBytes: 1024 * 1024,
			now: () => clock,
			prepare: async () => {
				prepares++;
				return {
					call() {
						clock++;
						return ++calls;
					},
					close() {
						closes++;
					}
				};
			}
		}),
		/64 MiB/
	);
	assert.deepEqual([calls, prepares, closes], [1, 1, 1]);
});

test('retained slots clear before disposal even when outside-timer validation rejects', async () => {
	let retained,
		closes = 0,
		clock = 0;
	await assert.rejects(
		collectCalls({
			measurement: { mode: 'single-call', warmup_ms: 1 },
			now: () => clock++,
			observe(outputs) {
				retained = outputs;
				throw new Error('invalid output');
			},
			prepare: async () => ({
				call: () => ({}),
				close() {
					closes++;
					assert.ok(retained.every((value) => value === undefined));
				}
			})
		}),
		/invalid output/
	);
	assert.equal(closes, 1);
});

test('every warmup and throughput output reaches the observer after its whole batch timer', async () => {
	let clock = 0,
		calls = 0,
		closes = 0;
	const batches = [];
	const result = await collectCalls({
		measurement: {
			mode: 'throughput',
			samples: 5,
			warmup_ms: 1,
			target_sample_ms: 3,
			measurement_ms: 100
		},
		now: () => clock,
		outputBytes: 4,
		observe: (outputs) => {
			batches.push([...outputs]);
			clock += 100;
		},
		prepare: async () => ({
			call() {
				clock++;
				return ++calls === 3 ? 'B' : 'A';
			},
			close() {
				closes++;
			}
		})
	});
	assert.deepEqual(batches, [
		['A'],
		['A', 'B', 'A'],
		['A', 'A', 'A'],
		['A', 'A', 'A'],
		['A', 'A', 'A'],
		['A', 'A', 'A']
	]);
	assert.deepEqual(result.sample_ns, Array(5).fill(1e6));
	assert.equal(result.iterations_per_sample, 3);
	assert.equal(calls, 16);
	assert.equal(closes, 6);
});

test('every initialization probe reaches the observer outside its timer and before disposal', async () => {
	let clock = 0,
		calls = 0,
		observed = 0;
	const values = [];
	const result = await collectInitializations({
		measurement: { samples: 5, warmup_ms: 1, measurement_ms: 100 },
		now: () => clock,
		outputBytes: 4,
		create: async () => {
			clock++;
			return {
				dispose() {
					assert.equal(observed, calls);
				}
			};
		},
		probe: () => (++calls === 3 ? 'B' : 'A'),
		observe: (outputs) => {
			values.push(...outputs);
			observed++;
			clock += 100;
		}
	});
	assert.deepEqual(values, ['A', 'A', 'B', 'A', 'A', 'A']);
	assert.deepEqual(result.sample_ns, Array(5).fill(1e6));
});

test('latency keeps a zero-duration call without retry, batching, or clamping', () => {
	let calls = 0;
	const result = timeCalls(
		() => ++calls,
		1,
		() => 10
	);
	assert.deepEqual(result, { output: 1, elapsed: 0, perCallNs: 0 });
	assert.equal(calls, 1);
});

test('throughput records its exact batch count and per-call duration', () => {
	let calls = 0;
	let clock = 0;
	const result = timeCalls(
		() => ++calls,
		4,
		() => (clock += 2)
	);
	assert.deepEqual(result, { output: 4, elapsed: 2, perCallNs: 500_000 });
	assert.equal(calls, 4);
});

test('warmup precedes a fresh preparation for every measured sample', async () => {
	const events = [];
	let clock = 0;
	let sequence = 0;
	const result = await collectCalls({
		measurement: { mode: 'single-call', warmup_ms: 6, samples: 5, measurement_ms: 0 },
		now: () => clock++,
		prepare: async () => {
			const id = ++sequence;
			events.push(`prepare ${id}`);
			return { call: () => events.push(`call ${id}`), close: () => events.push(`close ${id}`) };
		}
	});
	assert.equal(result.warmup_iterations, 2);
	assert.equal(result.iterations_per_sample, 1);
	assert.deepEqual(result.sample_ns, [1e6, 1e6, 1e6, 1e6, 1e6]);
	assert.deepEqual(
		events,
		Array.from({ length: 7 }, (_, i) => [
			`prepare ${i + 1}`,
			`call ${i + 1}`,
			`close ${i + 1}`
		]).flat()
	);
});

test('a stalled clock fails bounded warmup without inventing durations', async () => {
	let calls = 0;
	await assert.rejects(
		collectCalls({
			measurement: { mode: 'single-call', warmup_ms: 1, samples: 5 },
			now: () => 0,
			warmupAttemptLimit: 3,
			prepare: async () => ({ call: () => ++calls, close() {} })
		}),
		/Insufficient timer resolution/
	);
	assert.equal(calls, 3);
});

test('initialization awaits exactly one factory and leaves disposal outside timing', async () => {
	const events = [];
	let clock = 0;
	const instance = { dispose: () => events.push('dispose') };
	const result = await timeInitialization(
		async () => {
			events.push('create');
			return instance;
		},
		() => {
			events.push('clock');
			return clock++;
		}
	);
	assert.equal(result.elapsed, 1);
	assert.deepEqual(events, ['clock', 'create', 'clock']);
	result.instance.dispose();
	assert.equal(events.at(-1), 'dispose');
});

test('initialization collector probes and disposes each successful instance outside its timer', async () => {
	let clock = 0;
	let creates = 0;
	const events = [];
	const result = await collectInitializations({
		measurement: { samples: 5, warmup_ms: 1, measurement_ms: 0 },
		now: () => {
			events.push('clock');
			return clock++;
		},
		create: async () => {
			creates++;
			events.push('create');
			return { dispose: () => events.push('dispose') };
		},
		probe: () => {
			events.push('probe');
			return 'durable output';
		}
	});
	assert.equal(creates, 6);
	assert.equal(result.warmup_iterations, 1);
	assert.equal(result.output, 'durable output');
	assert.deepEqual(result.sample_ns, [1e6, 1e6, 1e6, 1e6, 1e6]);
	for (let index = 0; index < events.length; index++) {
		if (events[index] === 'create')
			assert.deepEqual(events.slice(index - 1, index + 4), [
				'clock',
				'create',
				'clock',
				'probe',
				'dispose'
			]);
	}
});

test('call failure still closes its prepared instance', async () => {
	let closed = 0;
	await assert.rejects(
		collectCalls({
			measurement: { warmup_ms: 1 },
			now: () => 0,
			prepare: async () => ({
				call: () => {
					throw new Error('operation failed');
				},
				close: () => closed++
			})
		}),
		/operation failed/
	);
	assert.equal(closed, 1);
});
