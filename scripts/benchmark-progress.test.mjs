import assert from 'node:assert/strict';
import test from 'node:test';
import { progressProbe } from './benchmark-progress.mjs';

test('progress observation accepts measurable stages and final completion', () => {
	const probe = progressProbe();
	probe.onProgress({ stage: 'prepare' });
	probe.onProgress({ stage: 'resize', completed: 3, total: 5 });
	probe.onProgress({ stage: 'complete' });
	assert.equal(probe.verify(), 3);
});

test('sample reset cannot inherit a prior completion', () => {
	const probe = progressProbe();
	probe.onProgress({ stage: 'complete', completed: 1, total: 1 });
	assert.equal(probe.verify(), 1);
	probe.reset();
	assert.throws(() => probe.verify(), /count=0/);
	probe.onProgress({ stage: 'prepare' });
	assert.throws(() => probe.verify(), /last=prepare/);
});

test('a later valid event cannot hide an earlier invalid event', () => {
	for (const event of [
		{ stage: 'unknown' },
		{ stage: 'quantize', completed: 2, total: 1 },
		{ stage: 'quantize', completed: -1 },
		{ stage: 'quantize', total: NaN },
		{ stage: 'quantize', completed: 0.5 }
	]) {
		const probe = progressProbe();
		probe.onProgress(event);
		probe.onProgress({ stage: 'complete' });
		assert.throws(() => probe.verify(), /invalid=true/);
	}
});

test('completion is the final event, including repeated completion', () => {
	for (const stage of ['complete', 'prepare']) {
		const probe = progressProbe();
		probe.onProgress({ stage: 'complete' });
		probe.onProgress({ stage });
		assert.throws(() => probe.verify(), /invalid=true/);
	}
});
