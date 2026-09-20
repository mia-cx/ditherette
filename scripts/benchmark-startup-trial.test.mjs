import assert from 'node:assert/strict';
import test from 'node:test';
import { warmProcessTrial } from './benchmark-stage-trial-fixture.mjs';
import { events, failInitialization, requireThreadPolicy } from './benchmark-stage-cache-fixture.mjs';

const startupTrial = (preparation, role, threads, configureFixture) => warmProcessTrial({
	configureFixture,
	configure(trial) {
		trial.role = role;
		trial.reference_output = trial.prime_reference_output;
		delete trial.prime_reference_output;
		Object.assign(trial.case.measurement, { scope: 'initialization', application_cache: 'not-applicable' });
		Object.assign(trial.case.browser, {
			operation: { operation: 'resize-nearest', anchor: 'center' },
			preparation, cache: 'none', threads
		});
	}
});

test('required startup failure aborts without fallback or another sample', async () => {
	await assert.rejects(startupTrial('initialization-compiled', 'candidate', {
		accepted: 'required', candidate: 'required'
	}, () => failInitialization(3)), /injected required startup failure/);
	assert.equal(events.filter(e => e.type === 'initialize').length, 3);
	assert.ok(events.filter(e => e.type === 'initialize').every(e => e.threads === 'required'));
	assert.deepEqual(events.filter(e => e.type === 'create').map(e => e.id),
		events.filter(e => e.type === 'dispose').map(e => e.id));
});

test('actual initialization adapter passes each role policy to preload and every measured factory', async () => {
	for (const preparation of ['initialization-bytes', 'initialization-compiled']) {
		for (const [role, selected] of [['accepted', 'disabled'], ['candidate', 'required']]) {
			const { result, events, trial } = await startupTrial(preparation, role, {
				accepted: 'disabled', candidate: 'required'
			});
			assert.deepEqual(result.output, trial.reference_output);
			assert.deepEqual(result.sample_ns, Array(5).fill(1e6));
			const initializations = events.filter(e => e.type === 'initialize');
			assert.equal(initializations.length, 8); // Preload, preflight, warmup, five samples.
			assert.ok(initializations.every(e => e.threads === selected));
			assert.equal(initializations[0].wasm, 'bytes');
			assert.ok(initializations.slice(1).every(e => e.wasm === (preparation === 'initialization-bytes' ? 'bytes' : 'compiled')));
			assert.deepEqual(events.filter(e => e.type === 'create').map(e => e.id),
				events.filter(e => e.type === 'dispose').map(e => e.id));
			assert.equal(events.filter(e => e.type === 'resize').length, 7);
		}
	}
});

test('historical initialization stays explicitly scalar and invalid role policies fail before creation', async () => {
	const { events } = await startupTrial('initialization-bytes', 'candidate');
	assert.ok(events.filter(e => e.type === 'initialize').every(e => e.threads === 'disabled'));
	for (const threads of [null, {}, { accepted: 'disabled', candidate: 'sometimes' }]) {
		await assert.rejects(startupTrial('initialization-compiled', 'candidate', threads), /Thread policies/);
	}
});

test('host declarations cannot silently run the package in the page context', async () => {
	await assert.rejects(
		warmProcessTrial({
			configure(trial) {
				trial.case.browser.execution = 'host-worker';
			}
		}),
		/execution context differs/
	);
});

test('Process composition keeps each role initializer paired with its selected Wasm entry', async () => {
	for (const role of ['candidate', 'accepted']) {
		for (const backend of ['package', 'package-staged']) {
			const selected = role === 'candidate' ? 'required' : 'disabled';
			const wasm = `package/dist/wasm/${selected === 'required' ? 'threads' : 'scalar'}/fixture.wasm`;
			const fetched = [];
			const { result, events, trial } = await warmProcessTrial({
				configureFixture: () => requireThreadPolicy(selected),
				configure(trial) {
					trial.role = role;
					trial.browser.assets.entries.wasm = wasm;
					delete trial.prime_reference_output;
					trial.case.measurement.application_cache = 'not-applicable';
					Object.assign(trial.case.browser, {
						cache: 'none', preparation: 'fresh-instance',
						[role]: backend,
						threads: { accepted: 'disabled', candidate: 'required' }
					});
					const fetch = globalThis.fetch;
					globalThis.fetch = (url) => {
						fetched.push(url);
						return fetch(url);
					};
				}
			});
			assert.deepEqual(fetched, [`file:///${wasm}`, `file:///${wasm}`]);
			assert.ok(events.filter(e => e.type === 'initialize').every(e => e.threads === selected));
			assert.equal(events.filter(e => e.type === 'initialize').length, 10);
			assert.equal(events.filter(e => e.type === 'process').length, backend === 'package' ? 7 : 1);
			assert.equal(events.filter(e => e.type === 'ditherAndQuantize').length, backend === 'package' ? 1 : 7);
			assert.deepEqual(result.output, trial.reference_output);
			assert.equal(result.role, role);
			assert.deepEqual(result.sample_ns, Array(5).fill(1e6));
			assert.equal(trial.case.browser[role], backend);
			assert.deepEqual(events.filter(e => e.type === 'create').map(e => e.id),
				events.filter(e => e.type === 'dispose').map(e => e.id));
		}
	}
});
