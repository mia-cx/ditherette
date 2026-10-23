import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	clearPersistedImages,
	clearPersistedImagesIfSourceMatches,
	clearPersistedProcessedImage,
	closeDb,
	loadProcessedImage,
	loadSourceImage,
	saveProcessedImage,
	saveSourceImageAndClearProcessed
} from './db';
import type { ProcessedImage, SourceImageRecord } from './types';
import { encodeIndexedPng } from './png';
import { encodeIndexedPngOffThread } from './png-worker';
import { buildOutputPyramid, releaseLevels } from './output-preview';

const nativeOpen = indexedDB.open.bind(indexedDB);
const connections: IDBDatabase[] = [];

function result<T>(request: IDBRequest<T>): Promise<T> {
	return new Promise((resolve, reject) => {
		request.addEventListener('success', () => resolve(request.result), { once: true });
		request.addEventListener('error', () => reject(request.error), { once: true });
	});
}

function source(updatedAt = 1): SourceImageRecord {
	return {
		blob: new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' }),
		name: 'source.png',
		width: 1,
		height: 1,
		type: 'image/png',
		updatedAt
	};
}

function processed(settingsHash = 'current'): ProcessedImage {
	return {
		width: 1,
		height: 1,
		indices: new Uint8Array([0]),
		palette: [
			{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true }
		],
		transparentIndex: -1,
		warnings: [],
		settingsHash,
		updatedAt: 1
	};
}

function observeConnections() {
	return vi.spyOn(indexedDB, 'open').mockImplementation((...args) => {
		const request = nativeOpen(...args);
		request.addEventListener('success', () => connections.push(request.result), { once: true });
		return request;
	});
}

beforeEach(async () => {
	await closeDb();
	await result(indexedDB.deleteDatabase('ditherette'));
});

afterEach(async () => {
	await closeDb();
	for (const db of connections) db.close();
	connections.length = 0;
	vi.restoreAllMocks();
	await result(indexedDB.deleteDatabase('ditherette'));
});

describe('IndexedDB connection ownership', () => {
	it('uses one open request and connection across eight serial operations', async () => {
		const open = observeConnections();
		await saveSourceImageAndClearProcessed(source());
		expect(await loadSourceImage()).toEqual(source());
		await saveProcessedImage(processed());
		expect(await loadProcessedImage()).toEqual(processed());
		await clearPersistedProcessedImage();
		await clearPersistedImages();
		expect(await loadSourceImage()).toBeUndefined();
		expect(await loadProcessedImage()).toBeUndefined();

		expect(open).toHaveBeenCalledExactlyOnceWith('ditherette', 1);
		expect(connections).toHaveLength(1);
		expect(Array.from(connections[0].objectStoreNames)).toEqual(['records']);
	});

	it('coalesces eight concurrent operations while preserving transaction creation order', async () => {
		const open = observeConnections();
		const pending = Promise.all([
			saveSourceImageAndClearProcessed(source()),
			loadSourceImage(),
			saveProcessedImage(processed()),
			loadProcessedImage(),
			clearPersistedProcessedImage(),
			clearPersistedImages(),
			loadSourceImage(),
			loadProcessedImage()
		]);
		expect(open).toHaveBeenCalledOnce();
		const values = await pending;
		expect(values[1]).toEqual(source());
		expect(values[3]).toEqual(processed());
		expect(values[6]).toBeUndefined();
		expect(values[7]).toBeUndefined();
		expect(open).toHaveBeenCalledOnce();
		expect(connections).toHaveLength(1);
	});

	it('explicitly closes without a close event and reopens with existing data', async () => {
		const open = observeConnections();
		await saveSourceImageAndClearProcessed(source());
		const old = connections[0];
		const closed = vi.fn();
		old.addEventListener('close', closed);
		await closeDb();
		expect(closed).not.toHaveBeenCalled();
		expect(() => old.transaction('records')).toThrow();
		expect(await loadSourceImage()).toEqual(source());

		// A late event from the released connection must not discard its replacement.
		old.dispatchEvent(new Event('close'));
		await loadSourceImage();
		expect(open).toHaveBeenCalledTimes(2);
		expect(connections).toHaveLength(2);
	});

	it('lets an acquired write finish when explicitly closed during opening', async () => {
		const open = observeConnections();
		const saving = saveSourceImageAndClearProcessed(source());
		await Promise.all([saving, closeDb()]);
		expect(await loadSourceImage()).toEqual(source());
		expect(open).toHaveBeenCalledTimes(2);
	});

	it('closes on a real version change so another context can upgrade', async () => {
		observeConnections();
		await saveSourceImageAndClearProcessed(source());
		const old = connections[0];
		const upgrade = indexedDB.open('ditherette', 2);
		const blocked = vi.fn();
		upgrade.onblocked = blocked;
		const upgraded = await result(upgrade);
		expect(blocked).not.toHaveBeenCalled();
		expect(() => old.transaction('records')).toThrow();
		expect(
			await result(upgraded.transaction('records').objectStore('records').get('source'))
		).toEqual(source());
		// The old app must report the incompatible version, rather than reuse the closed connection.
		await expect(loadSourceImage()).rejects.toMatchObject({ name: 'VersionError' });
	});

	it('reopens after another context deletes the isolated database', async () => {
		const open = observeConnections();
		await saveSourceImageAndClearProcessed(source());
		await result(indexedDB.deleteDatabase('ditherette'));
		expect(await loadSourceImage()).toBeUndefined();
		expect(open).toHaveBeenCalledTimes(2);
	});

	it('forgets an abnormally closed connection when its close notification arrives', async () => {
		const open = observeConnections();
		await saveSourceImageAndClearProcessed(source());
		const old = connections[0];
		old.close();
		// Public IndexedDB has no force-close API. Simulate only that notification.
		old.dispatchEvent(new Event('close'));
		expect(await loadSourceImage()).toEqual(source());
		expect(open).toHaveBeenCalledTimes(2);
	});

	it('reports a closed-connection write once and reacquires only for the next operation', async () => {
		const open = observeConnections();
		await saveSourceImageAndClearProcessed(source());
		connections[0].close();
		await expect(saveProcessedImage(processed())).rejects.toMatchObject({
			name: 'InvalidStateError'
		});
		expect(open).toHaveBeenCalledOnce();
		expect(await loadProcessedImage()).toBeUndefined();
		expect(await loadSourceImage()).toEqual(source());
		expect(open).toHaveBeenCalledTimes(2);
	});

	it('shares an aborted open rejection and recovers on the next acquisition', async () => {
		const open = observeConnections();
		open.mockImplementationOnce((...args) => {
			const request = nativeOpen(...args);
			queueMicrotask(() => {
				// Register after db.ts installs its schema handler, then abort the real upgrade.
				request.addEventListener('upgradeneeded', () => request.transaction!.abort());
			});
			return request;
		});
		const attempts = await Promise.allSettled([loadSourceImage(), loadProcessedImage()]);
		expect(attempts).toEqual([
			{ status: 'rejected', reason: expect.objectContaining({ name: 'AbortError' }) },
			{ status: 'rejected', reason: expect.objectContaining({ name: 'AbortError' }) }
		]);
		expect(open).toHaveBeenCalledOnce();
		await saveSourceImageAndClearProcessed(source());
		expect(await loadSourceImage()).toEqual(source());
		expect(open).toHaveBeenCalledTimes(2);
		expect(connections).toHaveLength(1);
	});

	it('recovers after indexedDB.open throws synchronously', async () => {
		const open = observeConnections();
		const failure = new DOMException('Storage unavailable', 'SecurityError');
		open.mockImplementationOnce(() => {
			throw failure;
		});
		await expect(loadSourceImage()).rejects.toBe(failure);
		expect(await loadSourceImage()).toBeUndefined();
		expect(open).toHaveBeenCalledTimes(2);
	});
});

describe('processed persistence admission and cancellation', () => {
	it('keeps an oversized output drawable and exportable after skipping persistence', async () => {
		const large = {
			...processed('large'),
			width: 4096,
			height: 4097,
			indices: new Uint8Array(4096 * 4097)
		};
		await saveProcessedImage(large);
		expect(await loadProcessedImage()).toBeUndefined();
		const levels = buildOutputPyramid(large);
		try {
			expect([levels[0].width, levels[0].height]).toEqual([4096, 4097]);
			const blob = await encodeIndexedPngOffThread(large);
			const header = new DataView(await blob.slice(0, 24).arrayBuffer());
			expect(header.getUint32(16)).toBe(4096);
			expect(header.getUint32(20)).toBe(4097);
			expect(large.indices.byteLength).toBe(4096 * 4097);
			expect(large.indices.at(-1)).toBe(0);
		} finally {
			releaseLevels(levels);
		}
	});

	it('skips oversized backing buffers, clears old output, and leaves export pixels intact', async () => {
		await saveProcessedImage(processed());
		const buffer = new Uint8Array(16 * 1024 * 1024 + 1);
		const large = { ...processed('large'), indices: buffer.subarray(0, 1) };
		const png = new Uint8Array(await encodeIndexedPng(large).arrayBuffer());
		const put = vi.spyOn(IDBObjectStore.prototype, 'put');
		await saveProcessedImage(large);
		expect(await loadProcessedImage()).toBeUndefined();
		expect(put).not.toHaveBeenCalled();
		expect(large.indices[0]).toBe(0);
		expect(large.indices.buffer.byteLength).toBe(buffer.byteLength);
		expect(new Uint8Array(await encodeIndexedPng(large).arrayBuffer())).toEqual(png);
	});

	it.each([-1, 0, 1])('admits processed payloads at the byte limit plus %i', async (difference) => {
		// Fixture metadata is 294 bytes under the documented accounting policy.
		const backing = new Uint8Array(16 * 1024 * 1024 - 294 + difference);
		const image = { ...processed(), indices: backing.subarray(0, 1) };
		await saveProcessedImage(image);
		const loaded = await loadProcessedImage();
		if (difference > 0) expect(loaded).toBeUndefined();
		else {
			expect(loaded?.settingsHash).toBe('current');
			expect(loaded?.indices.buffer.byteLength).toBe(backing.byteLength);
		}
	});

	it.each(['warnings', 'tags'])(
		'rejects oversized %s before cloning the processed payload',
		async (field) => {
			const put = vi.spyOn(IDBObjectStore.prototype, 'put');
			const record = processed();
			const largeText = ['x'.repeat(8 * 1024 * 1024)];
			if (field === 'warnings') record.warnings = largeText;
			else record.palette[0].tags = largeText;
			await saveProcessedImage(record);
			expect(put).not.toHaveBeenCalled();
			expect(await loadProcessedImage()).toBeUndefined();
		}
	);

	it('drops previously persisted oversized outputs so restore can regenerate', async () => {
		await loadSourceImage();
		const db = await result(nativeOpen('ditherette', 1));
		connections.push(db);
		const backing = new Uint8Array(16 * 1024 * 1024 + 1);
		await result(
			db
				.transaction('records', 'readwrite')
				.objectStore('records')
				.put(
					{
						...processed(),
						indices: backing.subarray(0, 1)
					},
					'processed'
				)
		);
		expect(await loadProcessedImage()).toBeUndefined();
		expect(
			await result(db.transaction('records').objectStore('records').get('processed'))
		).toBeUndefined();
	});

	it.each(['aborted', 'superseded'])(
		'does not clone a save %s while the connection opens',
		async (reason) => {
			await saveProcessedImage(processed('committed'));
			await closeDb();
			const controller = new AbortController();
			let current = true;
			const put = vi.spyOn(IDBObjectStore.prototype, 'put');
			const record = processed();
			if (reason === 'aborted') record.warnings = ['x'.repeat(8 * 1024 * 1024)];
			const saving = saveProcessedImage(record, {
				signal: controller.signal,
				isCurrent: () => current
			});
			if (reason === 'aborted') controller.abort();
			else current = false;
			await saving;
			expect(put).not.toHaveBeenCalled();
			expect((await loadProcessedImage())?.settingsHash).toBe('committed');
		}
	);

	it('aborts a superseded save after request success and preserves the last committed output', async () => {
		await saveProcessedImage(processed('committed'));
		const controller = new AbortController();
		const put = IDBObjectStore.prototype.put;
		vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementation(function (
			this: IDBObjectStore,
			...args
		) {
			const request = put.apply(this, args);
			request.addEventListener('success', () => controller.abort(), { once: true });
			return request;
		});
		await expect(
			saveProcessedImage(processed('superseded'), {
				signal: controller.signal,
				isCurrent: () => !controller.signal.aborted
			})
		).rejects.toThrow('IndexedDB transaction aborted');
		expect((await loadProcessedImage())?.settingsHash).toBe('committed');
	});
});

const writePaths = [
	{ name: 'processed save', write: () => saveProcessedImage(processed('new')) },
	{ name: 'source replacement', write: () => saveSourceImageAndClearProcessed(source(2)) },
	{ name: 'conditional rollback', write: () => clearPersistedImagesIfSourceMatches(source()) }
];

describe('IndexedDB transaction durability', () => {
	it.each(writePaths)('$name resolves only after transaction completion', async ({ write }) => {
		await saveSourceImageAndClearProcessed(source());
		await saveProcessedImage(processed());
		let settled = false;
		const events: string[] = [];
		const settledAtRequestSuccess: boolean[] = [];
		function observe<T>(request: IDBRequest<T>) {
			request.addEventListener('success', () => {
				events.push('request');
				queueMicrotask(() => settledAtRequestSuccess.push(settled));
			});
			request.transaction!.addEventListener('complete', () => events.push('complete'));
			return request;
		}
		const put = IDBObjectStore.prototype.put;
		vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementation(function (
			this: IDBObjectStore,
			...args
		) {
			return observe(put.apply(this, args));
		});
		const get = IDBObjectStore.prototype.get;
		vi.spyOn(IDBObjectStore.prototype, 'get').mockImplementation(function (
			this: IDBObjectStore,
			...args
		) {
			return observe(get.apply(this, args));
		});

		await write().then(() => {
			settled = true;
			events.push('resolved');
		});
		expect(settledAtRequestSuccess).toEqual([false]);
		expect(events).toEqual(['request', 'complete', 'resolved']);
	});

	it.each(writePaths)(
		'$name rejects an abort after request success without retrying',
		async ({ write }) => {
			const open = observeConnections();
			await saveSourceImageAndClearProcessed(source());
			await saveProcessedImage(processed());
			let successfulRequests = 0;
			function abortAfterSuccess<T>(request: IDBRequest<T>) {
				queueMicrotask(() => {
					// Run after the application's success handler has queued its remaining writes.
					request.addEventListener('success', () => {
						successfulRequests++;
						request.transaction!.abort();
					});
				});
				return request;
			}
			const put = IDBObjectStore.prototype.put;
			const putSpy = vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementation(function (
				this: IDBObjectStore,
				...args
			) {
				return abortAfterSuccess(put.apply(this, args));
			});
			const get = IDBObjectStore.prototype.get;
			const getSpy = vi.spyOn(IDBObjectStore.prototype, 'get').mockImplementation(function (
				this: IDBObjectStore,
				...args
			) {
				return abortAfterSuccess(get.apply(this, args));
			});

			await expect(write()).rejects.toThrow(/IndexedDB transaction (aborted|failed)/);
			expect(successfulRequests).toBe(1);
			putSpy.mockRestore();
			getSpy.mockRestore();
			expect(await loadSourceImage()).toEqual(source());
			expect(await loadProcessedImage()).toEqual(processed());
			expect(open).toHaveBeenCalledOnce();
		}
	);

	it.each([
		{
			name: 'processed save',
			write: () => saveProcessedImage(processed('new')),
			failure: { name: 'ConstraintError' }
		},
		{
			name: 'source replacement',
			write: () => saveSourceImageAndClearProcessed(source(2)),
			failure: { message: 'IndexedDB transaction failed' }
		}
	])(
		'$name propagates a real request error and preserves committed data',
		async ({ write, failure }) => {
			const open = observeConnections();
			await saveSourceImageAndClearProcessed(source());
			await saveProcessedImage(processed());
			// An add at an existing key fails asynchronously with ConstraintError.
			const add = IDBObjectStore.prototype.add;
			const put = vi.spyOn(IDBObjectStore.prototype, 'put').mockImplementation(function (
				this: IDBObjectStore,
				...args
			) {
				return add.apply(this, args);
			});
			await expect(write()).rejects.toMatchObject(failure);
			expect(put).toHaveBeenCalledOnce();
			put.mockRestore();
			expect(await loadSourceImage()).toEqual(source());
			expect(await loadProcessedImage()).toEqual(processed());
			expect(open).toHaveBeenCalledOnce();
		}
	);
});

describe('conditional source rollback', () => {
	it('atomically clears the matching source and its processed output', async () => {
		await saveSourceImageAndClearProcessed(source());
		await saveProcessedImage(processed());
		await clearPersistedImagesIfSourceMatches(source());
		expect(await loadSourceImage()).toBeUndefined();
		expect(await loadProcessedImage()).toBeUndefined();
	});

	it.each([
		{ name: 'new.png' },
		{ width: 2 },
		{ height: 2 },
		{ type: 'image/jpeg' },
		{ updatedAt: 2 },
		{ blob: new Blob(['different size'], { type: 'image/png' }) },
		{ blob: new Blob([new Uint8Array([1, 2, 3])], { type: 'image/jpeg' }) }
	])('keeps source and output when the source identity differs (%j)', async (difference) => {
		await saveSourceImageAndClearProcessed(source());
		await saveProcessedImage(processed());
		await clearPersistedImagesIfSourceMatches({ ...source(), ...difference });
		expect(await loadSourceImage()).toEqual(source());
		expect(await loadProcessedImage()).toEqual(processed());
	});

	it.each(['rollback first', 'replacement first'])(
		'keeps the newer source and settings when concurrent operations queue %s',
		async (order) => {
			await saveSourceImageAndClearProcessed(source());
			const rollback = () => clearPersistedImagesIfSourceMatches(source());
			const replacement = () => saveSourceImageAndClearProcessed(source(2));
			const operations =
				order === 'rollback first' ? [rollback, replacement] : [replacement, rollback];
			await Promise.all([...operations.map((run) => run()), saveProcessedImage(processed('new'))]);
			expect(await loadSourceImage()).toEqual(source(2));
			expect(await loadProcessedImage()).toEqual(processed('new'));
		}
	);

	it('clears an older output queued before a new source during cold acquisition', async () => {
		await Promise.all([
			saveProcessedImage(processed('old')),
			saveSourceImageAndClearProcessed(source(2))
		]);
		expect(await loadSourceImage()).toEqual(source(2));
		expect(await loadProcessedImage()).toBeUndefined();
	});

	it('leaves output alone when the source has already gone', async () => {
		await saveProcessedImage(processed());
		await clearPersistedImagesIfSourceMatches(source());
		expect(await loadProcessedImage()).toEqual(processed());
	});

	it('rejects invalid stored source data instead of deleting either record', async () => {
		observeConnections();
		await saveSourceImageAndClearProcessed(source());
		await saveProcessedImage(processed());
		const corrupted = { ...source(), name: null };
		await result(
			connections[0]
				.transaction('records', 'readwrite')
				.objectStore('records')
				.put(corrupted, 'source')
		);
		await expect(clearPersistedImagesIfSourceMatches(source())).rejects.toThrow(
			'Saved source image name'
		);
		expect(await loadProcessedImage()).toEqual(processed());
		expect(
			await result(connections[0].transaction('records').objectStore('records').get('source'))
		).toEqual(corrupted);
	});
});
