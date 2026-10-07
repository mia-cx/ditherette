import { describe, expect, it, vi } from 'vitest';
import { POST } from './+server';

const VALID = {
	version: 1,
	settings: {
		output: { width: 10, height: 10, cropped: false },
		dither: {},
		colorSpace: 'oklab',
		palette: {
			name: 'Test',
			colours: [{ rgb: [0, 0, 0], enabled: true }]
		},
		effects: []
	},
	fits: [],
	export: { width: 10, height: 10, format: 'png' }
};

const call = (body: BodyInit | string, events?: { prepare: ReturnType<typeof vi.fn> }) =>
	POST({
		request: new Request('https://ditherette.mia.cx/api/events', { method: 'POST', body }),
		platform: events && { env: { EVENTS: events } }
	} as never);

/** A fake D1 that records what prepare/bind/run chain was used. */
const fakeD1 = () => {
	const run = vi.fn(async () => ({ success: true }));
	const bind = vi.fn(() => ({ run }));
	const prepare = vi.fn(() => ({ bind }));
	return { prepare, bind, run };
};

describe('POST /api/events', () => {
	it('stores a valid event and answers 204', async () => {
		const events = fakeD1();
		const response = await call(JSON.stringify(VALID), events);
		expect(response.status).toBe(204);
		expect(events.prepare).toHaveBeenCalledOnce();
		const [receivedAt, version, payload] = events.bind.mock.calls[0]! as unknown as [
			string,
			number,
			string
		];
		expect(new Date(receivedAt).toISOString()).toBe(receivedAt);
		expect(version).toBe(1);
		expect(JSON.parse(payload)).toEqual(VALID);
		expect(events.run).toHaveBeenCalledOnce();
	});

	it('rejects malformed JSON and schema mismatches with 400', async () => {
		const events = fakeD1();
		expect((await call('{nope', events)).status).toBe(400);
		expect((await call('{"version":2}', events)).status).toBe(400);
		expect((await call(JSON.stringify({ ...VALID, version: 2 }), events)).status).toBe(400);
		expect(events.prepare).not.toHaveBeenCalled();
	});

	it('rejects a body over 64 KiB with 413', async () => {
		const events = fakeD1();
		const big = JSON.stringify({ ...VALID, fits: 'x'.repeat(70 * 1024) });
		expect((await call(big, events)).status).toBe(413);
		expect(events.prepare).not.toHaveBeenCalled();
	});

	it('answers 204 without storing when no D1 binding exists', async () => {
		const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
		const response = await call(JSON.stringify(VALID));
		expect(response.status).toBe(204);
		expect(warn).toHaveBeenCalled();
	});
});
