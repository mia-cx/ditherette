import assert from 'node:assert/strict';
import test from 'node:test';
import { WorkerPool } from '../dist/worker-pool.js';

function setup(t, failure) {
	const events = [];
	const workers = [];
	class Worker extends EventTarget {
		constructor() {
			super();
			if (failure === 'constructor' && workers.length === 1) throw new Error('constructor');
			workers.push(this);
		}
		postMessage(message) {
			if (message.type === 'ditherette-worker-init') queueMicrotask(() => {
				this.dispatchEvent(new MessageEvent('message', { data: {
					type: failure === 'ready' ? 'ditherette-worker-error' : 'ditherette-worker-ready'
				} }));
			});
			else if (failure === 'dispatch') throw new Error('dispatch');
		}
		terminate() { events.push('terminate'); }
	}
	const previous = Object.getOwnPropertyDescriptor(globalThis, 'Worker');
	Object.defineProperty(globalThis, 'Worker', { configurable: true, value: Worker });
	t.after(() => {
		if (previous) Object.defineProperty(globalThis, 'Worker', previous);
		else delete globalThis.Worker;
	});
	const builder = {
		numThreads: () => 2,
		receiver: () => 123,
		build() { events.push('build'); if (failure === 'build') throw new Error('build'); },
		free() { events.push('free'); }
	};
	return { events, builder, abandon() { events.push('abandon'); } };
}

test('pool frees the borrowed receiver only after successful priming and releases each worker once', async (t) => {
	const { events, builder, abandon } = setup(t);
	const pool = new WorkerPool();
	await pool.start({}, {}, builder, abandon);
	assert.deepEqual(events, ['build', 'free']);
	pool.dispose();
	pool.dispose();
	assert.deepEqual(events, ['build', 'free', 'terminate', 'terminate']);
});

for (const failure of ['constructor', 'ready', 'dispatch', 'build']) {
	test(`pool owns partial workers and preserves receiver lifetime after ${failure} failure`, async (t) => {
		const { events, builder, abandon } = setup(t, failure);
		const pool = new WorkerPool();
		await assert.rejects(pool.start({}, {}, builder, abandon));
		const count = failure === 'constructor' ? 1 : 2;
		assert.equal(events.filter((event) => event === 'terminate').length, count);
		assert.equal(events.at(-1), failure === 'dispatch' || failure === 'build' ? 'abandon' : 'free');
		const before = [...events];
		pool.dispose();
		assert.deepEqual(events, before);
	});
}
