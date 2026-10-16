import { validateProcessedImage, validateSourceImageRecord } from './schemas';
import type { ProcessedImage, SourceImageRecord } from './types';

const DB_NAME = 'ditherette';
const DB_VERSION = 1;
const STORE = 'records';
const SOURCE_KEY = 'source';
const PROCESSED_KEY = 'processed';

// At most 16 MiB of structured-clone inputs per processed save: the entire index backing
// buffer (subarrays clone it too), UTF-16 strings, and conservative fixed metadata allowances.
// Browser storage overhead is separate. Skipped outputs stay in memory for preview/export;
// deleting the previous saved output makes restore regenerate from the persisted source.
const MAX_PROCESSED_SAVE_BYTES = 16 * 1024 * 1024;

function canPersistProcessedImage(record: ProcessedImage) {
	let bytes = record.indices.buffer.byteLength + 128 + record.settingsHash.length * 2;
	for (const warning of record.warnings) bytes += 32 + warning.length * 2;
	for (const color of record.palette) {
		bytes += 128 + (color.name.length + color.key.length) * 2;
		for (const tag of color.tags ?? []) bytes += 32 + tag.length * 2;
	}
	return bytes <= MAX_PROCESSED_SAVE_BYTES;
}

type SaveGuard = { signal: AbortSignal; isCurrent: () => boolean };

let connection: Promise<IDBDatabase> | undefined;

function openDb(): Promise<IDBDatabase> {
	if (connection) return connection;
	const opening = new Promise<IDBDatabase>((resolve, reject) => {
		const request = indexedDB.open(DB_NAME, DB_VERSION);
		request.onupgradeneeded = () => {
			request.result.createObjectStore(STORE);
		};
		request.onerror = () => reject(request.error ?? new Error('Failed to open IndexedDB'));
		request.onsuccess = () => {
			const db = request.result;
			const forget = () => {
				if (connection === opening) connection = undefined;
			};
			db.onclose = forget;
			db.onversionchange = () => {
				forget();
				db.close();
			};
			resolve(db);
		};
	});
	connection = opening;
	void opening.catch(() => {
		if (connection === opening) connection = undefined;
	});
	return opening;
}

/** Release shared ownership. Existing transactions finish; later operations open again. */
export async function closeDb(): Promise<void> {
	const closing = connection;
	connection = undefined;
	if (closing) (await closing).close();
}

async function openTransaction(mode: IDBTransactionMode): Promise<IDBTransaction> {
	const opening = openDb();
	const db = await opening;
	try {
		return db.transaction(STORE, mode);
	} catch (error) {
		// A connection can close before its close event arrives. Fail this operation without replaying it.
		if (
			error instanceof DOMException &&
			error.name === 'InvalidStateError' &&
			connection === opening
		)
			connection = undefined;
		throw error;
	}
}

async function withStore<T>(
	mode: IDBTransactionMode,
	run: (store: IDBObjectStore) => IDBRequest<T>
): Promise<T> {
	const tx = await openTransaction(mode);
	return new Promise<T>((resolve, reject) => {
		let result: T;
		const request = run(tx.objectStore(STORE));
		request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'));
		request.onsuccess = () => {
			result = request.result;
		};
		tx.onerror = () => reject(tx.error ?? new Error('IndexedDB transaction failed'));
		tx.onabort = () => reject(tx.error ?? new Error('IndexedDB transaction aborted'));
		tx.oncomplete = () => resolve(result);
	});
}

async function withWriteTransaction(
	run: (store: IDBObjectStore) => IDBRequest | void,
	guard?: SaveGuard
): Promise<void> {
	const tx = await openTransaction('readwrite');
	await new Promise<void>((resolve, reject) => {
		const abort = () => {
			try {
				tx.abort();
			} catch (error) {
				// A committed transaction cannot be revoked while its completion event is queued.
				if (!(error instanceof DOMException && error.name === 'InvalidStateError')) throw error;
			}
		};
		const finish = (error?: unknown) => {
			guard?.signal.removeEventListener('abort', abort);
			if (error) reject(error);
			else resolve();
		};
		tx.onerror = () => finish(tx.error ?? new Error('IndexedDB transaction failed'));
		tx.onabort = () => finish(tx.error ?? new Error('IndexedDB transaction aborted'));
		// Opening may have waited behind an upload, clear, or newer settings. Check before cloning.
		try {
			if (guard && (guard.signal.aborted || !guard.isCurrent())) return;
			guard?.signal.addEventListener('abort', abort, { once: true });
			const request = run(tx.objectStore(STORE));
			if (request)
				request.onerror = () => finish(request.error ?? new Error('IndexedDB request failed'));
		} catch (error) {
			abort();
			finish(error);
		} finally {
			tx.oncomplete = () => finish();
		}
	});
}

function sameSourceRecord(left: SourceImageRecord, right: SourceImageRecord) {
	return (
		left.name === right.name &&
		left.width === right.width &&
		left.height === right.height &&
		left.type === right.type &&
		left.updatedAt === right.updatedAt &&
		left.blob.size === right.blob.size &&
		left.blob.type === right.blob.type
	);
}

export async function saveSourceImageAndClearProcessed(record: SourceImageRecord) {
	const safeRecord = validateSourceImageRecord(record);
	await withWriteTransaction((store) => {
		store.put(safeRecord, SOURCE_KEY);
		store.delete(PROCESSED_KEY);
	});
}

export async function clearPersistedImagesIfSourceMatches(record: SourceImageRecord) {
	const safeRecord = validateSourceImageRecord(record);
	const tx = await openTransaction('readwrite');
	await new Promise<void>((resolve, reject) => {
		const store = tx.objectStore(STORE);
		const request = store.get(SOURCE_KEY);
		request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'));
		request.onsuccess = () => {
			try {
				const current = request.result;
				if (current === undefined) return;
				if (!sameSourceRecord(validateSourceImageRecord(current), safeRecord)) return;
				store.delete(SOURCE_KEY);
				store.delete(PROCESSED_KEY);
			} catch (error) {
				tx.abort();
				reject(error instanceof Error ? error : new Error('IndexedDB rollback failed'));
			}
		};
		tx.onerror = () => reject(tx.error ?? new Error('IndexedDB transaction failed'));
		tx.onabort = () => reject(tx.error ?? new Error('IndexedDB transaction aborted'));
		tx.oncomplete = () => resolve();
	});
}

export async function loadSourceImage(): Promise<SourceImageRecord | undefined> {
	const record = await withStore('readonly', (store) => store.get(SOURCE_KEY));
	return record === undefined ? undefined : validateSourceImageRecord(record);
}

/** Save admitted worker output, or clear old output when over budget. Never mutate its pixels. */
export function saveProcessedImage(record: ProcessedImage, guard?: SaveGuard) {
	// Keep oversized buffers out of the asynchronous save closure, including while opening DB.
	return saveAdmittedProcessedImage(canPersistProcessedImage(record) ? record : undefined, guard);
}

function saveAdmittedProcessedImage(record: ProcessedImage | undefined, guard?: SaveGuard) {
	const release = () => {
		record = undefined;
	};
	// A blocked open must not retain each superseded output until IndexedDB responds.
	guard?.signal.addEventListener('abort', release, { once: true });
	if (guard?.signal.aborted) release();
	return withWriteTransaction((store) => {
		const request = record ? store.put(record, PROCESSED_KEY) : store.delete(PROCESSED_KEY);
		release();
		return request;
	}, guard).finally(() => {
		release();
		guard?.signal.removeEventListener('abort', release);
	});
}

export async function clearPersistedProcessedImage() {
	await withStore('readwrite', (store) => store.delete(PROCESSED_KEY));
}

export async function loadProcessedImage(): Promise<ProcessedImage | undefined> {
	let image: ProcessedImage | undefined;
	let validationError: unknown;
	await withWriteTransaction((store) => {
		const request = store.get(PROCESSED_KEY);
		request.onsuccess = () => {
			try {
				if (request.result === undefined) return;
				const record = validateProcessedImage(request.result);
				if (canPersistProcessedImage(record)) image = record;
				else store.delete(PROCESSED_KEY);
			} catch (error) {
				// Propagate validation failures through the transaction's failure path.
				request.transaction!.abort();
				validationError = error;
			}
		};
	}).catch((error: unknown) => {
		throw validationError ?? error;
	});
	return image;
}

export async function clearPersistedImages() {
	await withWriteTransaction((store) => {
		store.delete(SOURCE_KEY);
		store.delete(PROCESSED_KEY);
	});
}

export function sourceMetaFromRecord(record: SourceImageRecord) {
	return {
		name: record.name,
		width: record.width,
		height: record.height,
		type: record.type,
		updatedAt: record.updatedAt
	};
}
