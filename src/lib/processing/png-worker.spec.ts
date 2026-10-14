import { afterEach, describe, expect, it, vi } from 'vitest';
import { encodeIndexedPngOffThread } from './png-worker';
import type { PngWorkerRequest, PngWorkerResponse } from './png-worker';
import type { ProcessedImage } from './types';

const image: ProcessedImage = {
	width: 2,
	height: 1,
	indices: new Uint8Array([99, 0, 1, 99]).subarray(1, 3),
	palette: [
		{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
		{ name: 'Clear', key: 'transparent', kind: 'transparent', enabled: true }
	],
	transparentIndex: 1,
	warnings: [],
	settingsHash: 'test',
	updatedAt: 0
};

class TestWorker {
	static latest: TestWorker;
	onmessage: ((event: { data: PngWorkerResponse }) => void) | null = null;
	onerror: (() => void) | null = null;
	onmessageerror: (() => void) | null = null;
	terminate = vi.fn();
	request?: PngWorkerRequest;
	constructor() {
		TestWorker.latest = this;
	}
	postMessage(request: PngWorkerRequest, transfer: Transferable[]) {
		this.request = structuredClone(request, { transfer });
	}
}

afterEach(() => vi.unstubAllGlobals());

describe('PNG worker client', () => {
	it('transfers only an owned copy and terminates after completion', async () => {
		vi.stubGlobal('Worker', TestWorker);
		const pending = encodeIndexedPngOffThread(image);
		const worker = TestWorker.latest;
		expect(worker.request?.image.indices).toEqual(new Uint8Array([0, 1]));
		expect(worker.request?.image.indices.buffer.byteLength).toBe(2);
		expect(image.indices).toEqual(new Uint8Array([0, 1]));
		expect(image.indices.buffer.byteLength).toBe(4);
		const blob = new Blob(['PNG'], { type: 'image/png' });
		worker.onmessage?.({ data: { blob } });
		await expect(pending).resolves.toBe(blob);
		expect(worker.terminate).toHaveBeenCalledOnce();
		expect(worker.onmessage).toBeNull();
	});

	it('aborts an active worker and rejects instead of publishing a late response', async () => {
		vi.stubGlobal('Worker', TestWorker);
		const controller = new AbortController();
		const pending = encodeIndexedPngOffThread(image, controller.signal);
		const worker = TestWorker.latest;
		controller.abort();
		await expect(pending).rejects.toMatchObject({ name: 'AbortError' });
		expect(worker.terminate).toHaveBeenCalledOnce();
		expect(worker.onmessage).toBeNull();
	});

	it('does not start an already cancelled request', async () => {
		const constructor = vi.fn();
		vi.stubGlobal('Worker', constructor);
		await expect(encodeIndexedPngOffThread(image, AbortSignal.abort())).rejects.toMatchObject({
			name: 'AbortError'
		});
		expect(constructor).not.toHaveBeenCalled();
	});

	it.each(['onerror', 'onmessageerror'] as const)('releases a worker on %s', async (event) => {
		vi.stubGlobal('Worker', TestWorker);
		const pending = encodeIndexedPngOffThread(image);
		TestWorker.latest[event]?.();
		await expect(pending).rejects.toThrow(/worker/i);
		expect(TestWorker.latest.terminate).toHaveBeenCalledOnce();
	});

	it('releases a worker when posting the request fails', async () => {
		vi.stubGlobal('Worker', TestWorker);
		vi.spyOn(TestWorker.prototype, 'postMessage').mockImplementationOnce(() => {
			throw new Error('Clone failed');
		});
		await expect(encodeIndexedPngOffThread(image)).rejects.toThrow('Clone failed');
		expect(TestWorker.latest.terminate).toHaveBeenCalledOnce();
	});

	it('rejects an encoder error and releases the worker', async () => {
		vi.stubGlobal('Worker', TestWorker);
		const pending = encodeIndexedPngOffThread(image);
		TestWorker.latest.onmessage?.({ data: { error: 'Missing palette entry' } });
		await expect(pending).rejects.toThrow('Missing palette entry');
		expect(TestWorker.latest.terminate).toHaveBeenCalledOnce();
	});
});
