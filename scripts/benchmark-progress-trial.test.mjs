import assert from 'node:assert/strict';
import test from 'node:test';
import { warmProcessTrial } from './benchmark-stage-trial-fixture.mjs';
import { events, failProgress } from './benchmark-stage-cache-fixture.mjs';

const progressTrial = (role = 'candidate', failure) =>
	warmProcessTrial({
		configure(trial) {
			trial.role = role;
			trial.case.measurement.application_cache = 'cold';
			Object.assign(trial.case.browser, {
				preparation: 'fresh-instance',
				cache: { roles: { accepted: 'image-stages', candidate: 'image-stages' } },
				progress: { accepted: 'disabled', candidate: 'enabled' }
			});
		},
		configureFixture() {
			failProgress(failure);
		}
	});

test('full trial enables ordinary callbacks per role and excludes staged Process comparison', async () => {
	for (const role of ['accepted', 'candidate']) {
		const { result, events, trial } = await progressTrial(role);
		assert.deepEqual(result.output, trial.reference_output);
		assert.deepEqual(result.sample_ns, Array(5).fill(1e6));
		assert.equal(result.warmup_iterations, 1);
		const callbackIds = events
			.filter((event) => event.type === 'callback')
			.map((event) => event.id);
		assert.equal(callbackIds.length, role === 'accepted' ? 0 : 7);
		const stagedId = events.find((event) => event.type === 'ditherAndQuantize').id;
		assert.ok(!callbackIds.includes(stagedId));
		assert.deepEqual(
			events.filter((e) => e.type === 'create').map((e) => e.id),
			events.filter((e) => e.type === 'dispose').map((e) => e.id)
		);
	}
});

test('every sample resets and verifies progress without inheriting preflight or warmup completion', async () => {
	await assert.rejects(
		progressTrial('candidate', { at: 3, kind: 'incomplete' }),
		/count=1.*last=prepare/
	);
	assert.equal(events.filter((e) => e.type === 'callback').length, 3);
	assert.equal(
		events.filter((e) => e.type === 'create').length,
		events.filter((e) => e.type === 'dispose').length
	);
});

test('thrown callback work aborts its sample and disposes the active processor', async () => {
	await assert.rejects(
		progressTrial('candidate', { at: 3, kind: 'throw' }),
		/injected callback failure/
	);
	assert.equal(
		events.filter((e) => e.type === 'create').length,
		events.filter((e) => e.type === 'dispose').length
	);
});
