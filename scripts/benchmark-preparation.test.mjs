import assert from 'node:assert/strict';
import test from 'node:test';
import { preflightOperation, primeChangedSource } from './benchmark-public-page.mjs';

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
