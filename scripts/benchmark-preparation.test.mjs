import assert from 'node:assert/strict';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import {
	assertMeasuredSource,
	prepareOperation,
	preflightOperation,
	primeChangedSource
} from './benchmark-public-page.mjs';
import { collectCalls } from './benchmark-public-timing.mjs';
import { events, reset } from './benchmark-preparation-fixture.mjs';

test('warm priming changes only source bytes and restores the measured request on failure', () => {
	const data = new Uint8Array([10, 20, 30, 255]);
	const request = { source: { width: 1, height: 1, data }, settings: {} };
	let calls = 0;
	primeChangedSource(request, () => {
		calls++;
		assert.deepEqual([...request.source.data], [245, 20, 30, 255]);
		return { width: 1, height: 1, data: request.source.data.slice() };
	});
	assert.equal(calls, 1);
	assert.equal(request.source.data, data);
	assert.throws(
		() =>
			primeChangedSource(request, () => {
				throw new Error('failed');
			}),
		/failed/
	);
	assert.equal(request.source.data, data);
});

test('preflight observes a transient result before disposing its processor', async () => {
	const events = [];
	const output = { width: 1, height: 1, data: new Uint8Array([1, 2, 3, 255]) };
	const reference = {
		dimensions: { width: 1, height: 1 },
		pixels: { format: 'rgba8', data: [1, 2, 3, 255] },
		warnings: []
	};
	const result = await preflightOperation(
		{
			prepare: async () => ({ call: () => output, close: () => events.push('close') })
		},
		reference,
		() => events.push('observe')
	);
	assert.equal(result, undefined);
	assert.deepEqual(events, ['observe', 'close']);
});

test('sample observation rejects transient source mutation and closes before another call', async () => {
	const measured = [1, 2, 3, 255];
	const request = { source: { data: new Uint8Array(measured) } };
	let clock = 0;
	let calls = 0;
	let closes = 0;
	await assert.rejects(
		collectCalls({
			measurement: { mode: 'single-call', warmup_ms: 1, samples: 5, measurement_ms: 10 },
			now: () => clock++,
			outputBytes: 4,
			prepare: async () => ({
				call: () => {
					calls++;
					request.source.data[0] = calls === 2 ? 99 : measured[0];
					return { width: 1, height: 1, data: new Uint8Array(measured) };
				},
				close: () => closes++
			}),
			observe: () => assertMeasuredSource(request, measured)
		}),
		/mutated source/
	);
	assert.equal(calls, 2);
	assert.equal(closes, 2);
});

test('actual preparation adapter preserves cold/warm inputs and cleans up failed priming', async () => {
	const previousLocation = Object.getOwnPropertyDescriptor(globalThis, 'location');
	const previousFetch = globalThis.fetch;
	Object.defineProperty(globalThis, 'location', {
		configurable: true,
		value: { href: 'file:///' }
	});
	globalThis.fetch = async () => new Response(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]));
	const trial = {
		role: 'candidate',
		browser: {
			assets: {
				entries: {
					package: fileURLToPath(
						new URL('./benchmark-preparation-fixture.mjs', import.meta.url)
					).slice(1),
					wasm: 'unused.wasm'
				}
			}
		},
		case: {
			source: { width: 1, height: 1 },
			rgba: [1, 2, 3, 255],
			identity: { output: { width: 1, height: 1 } },
			measurement: { mode: 'single-call', scope: 'complete-call', application_cache: 'cold' },
			browser: {
				operation: { operation: 'resize-nearest', anchor: 'center' },
				accepted: 'package',
				candidate: 'package',
				preparation: 'fresh-instance',
				cache: { roles: { accepted: 'uncached', candidate: 'preparation' } }
			}
		}
	};
	try {
		for (const warm of [false, true]) {
			reset();
			trial.case.measurement.application_cache = warm ? 'warm' : 'cold';
			trial.case.browser.preparation = warm ? 'primed-instance' : 'fresh-instance';
			const operation = await prepareOperation(trial);
			try {
				for (let index = 0; index < 2; index++) {
					const prepared = await operation.prepare();
					try {
						assert.deepEqual([...prepared.call().data], trial.case.rgba);
					} finally {
						prepared.close();
					}
				}
			} finally {
				operation.close();
			}
			const calls = events.filter((event) => event.type === 'call');
			assert.deepEqual(
				calls.map((call) => call.id),
				warm ? [2, 2, 2] : [2, 3]
			);
			assert.deepEqual(
				calls.map((call) => call.bytes),
				warm
					? [[254, 2, 3, 255], trial.case.rgba, trial.case.rgba]
					: [trial.case.rgba, trial.case.rgba]
			);
			assert.equal(
				events.filter((event) => event.type === 'create').length,
				events.filter((event) => event.type === 'dispose').length
			);
		}
		reset(true);
		await assert.rejects(prepareOperation(trial), /fixture call failed/);
		assert.deepEqual(
			events.map((event) => event.type),
			['create', 'dispose', 'create', 'call', 'dispose']
		);
	} finally {
		globalThis.fetch = previousFetch;
		if (previousLocation) Object.defineProperty(globalThis, 'location', previousLocation);
		else delete globalThis.location;
	}
});
