import assert from 'node:assert/strict';
import test from 'node:test';
import { createRowBandProcessor } from './benchmark-row-policy.mjs';

function fixture(status = 0) {
	const calls = [];
	const instance = { exports: {
		privateThreadCount: () => 4,
		privateExecutionPolicy: (...args) => { calls.push(args); return status; }
	} };
	const wasm = { instantiate: async () => instance };
	const original = wasm.instantiate;
	let disposed = 0;
	const processor = { dispose() { disposed++; } };
	const create = async () => { await wasm.instantiate({}); return processor; };
	return { calls, instance, wasm, original, processor, create, disposed: () => disposed };
}
const policy = { stage: 'indexed', parameters: { height: 16, active_workers: 2 } };

test('sets only the real created instance and restores instantiation before measured methods', async () => {
	const f = fixture();
	const result = await createRowBandProcessor(f.create, policy, 8, f.wasm);
	assert.equal(result.processor, f.processor);
	assert.equal(f.wasm.instantiate, f.original);
	assert.deepEqual(f.calls, [[1, 16, 2, 4]]);
	assert.deepEqual(result.observation, { ...policy, pool_size: 4 });
	assert.equal(f.disposed(), 0);
});

test('setter failures dispose ownership and restore the global hook', async () => {
	const f = fixture(8);
	await assert.rejects(createRowBandProcessor(f.create, policy, 8, f.wasm), /status 8/);
	assert.equal(f.disposed(), 1);
	assert.equal(f.wasm.instantiate, f.original);
});

test('creation failures restore the hook without inventing processor ownership', async () => {
	const f = fixture();
	await assert.rejects(createRowBandProcessor(async () => { throw new Error('init failed'); }, policy, 8, f.wasm), /init failed/);
	assert.equal(f.disposed(), 0);
	assert.equal(f.wasm.instantiate, f.original);
});

test('missing developer exports, multiple instances, and excessive worker requests fail before timing', async () => {
	for (const fault of ['export', 'instances', 'workers']) {
		const f = fixture();
		if (fault === 'export') delete f.instance.exports.privateExecutionPolicy;
		const create = async () => { const p = await f.create(); if (fault === 'instances') await f.wasm.instantiate({}); return p; };
		const selected = fault === 'workers' ? { ...policy, parameters: { height: 16, active_workers: 8 } } : policy;
		await assert.rejects(createRowBandProcessor(create, selected, 8, f.wasm));
		assert.equal(f.disposed(), 1);
		assert.equal(f.wasm.instantiate, f.original);
	}
});
