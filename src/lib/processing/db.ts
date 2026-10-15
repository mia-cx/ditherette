import { validateProcessedImage, validateSourceImageRecord } from './schemas';
import type { ProcessedImage, SourceImageRecord } from './types';

const DB_NAME = 'ditherette';
const DB_VERSION = 1;
const STORE = 'records';
const SOURCE_KEY = 'source';
const PROCESSED_KEY = 'processed';

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

async function withWriteTransaction(run: (store: IDBObjectStore) => void): Promise<void> {
	const tx = await openTransaction('readwrite');
	await new Promise<void>((resolve, reject) => {
		run(tx.objectStore(STORE));
		tx.onerror = () => reject(tx.error ?? new Error('IndexedDB transaction failed'));
		tx.onabort = () => reject(tx.error ?? new Error('IndexedDB transaction aborted'));
		tx.oncomplete = () => resolve();
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

/** Save a result the worker response already validated. Loading validates it again. */
export async function saveProcessedImage(record: ProcessedImage) {
	await withStore('readwrite', (store) => store.put(record, PROCESSED_KEY));
}

export async function clearPersistedProcessedImage() {
	await withStore('readwrite', (store) => store.delete(PROCESSED_KEY));
}

export async function loadProcessedImage(): Promise<ProcessedImage | undefined> {
	const record = await withStore('readonly', (store) => store.get(PROCESSED_KEY));
	return record === undefined ? undefined : validateProcessedImage(record);
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
