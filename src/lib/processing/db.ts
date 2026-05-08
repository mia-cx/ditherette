import { validateProcessedImage, validateSourceImageRecord } from './schemas';
import type { ProcessedImage, SourceImageRecord } from './types';

const DB_NAME = 'ditherette';
const DB_VERSION = 1;
const STORE = 'records';
const SOURCE_KEY = 'source';
const PROCESSED_KEY = 'processed';

function openDb(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const request = indexedDB.open(DB_NAME, DB_VERSION);
		request.onupgradeneeded = () => {
			request.result.createObjectStore(STORE);
		};
		request.onerror = () => reject(request.error ?? new Error('Failed to open IndexedDB'));
		request.onsuccess = () => resolve(request.result);
	});
}

async function withStore<T>(
	mode: IDBTransactionMode,
	run: (store: IDBObjectStore) => IDBRequest<T>
): Promise<T> {
	const db = await openDb();
	try {
		return await new Promise<T>((resolve, reject) => {
			const tx = db.transaction(STORE, mode);
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
	} finally {
		db.close();
	}
}

async function withWriteTransaction(run: (store: IDBObjectStore) => void): Promise<void> {
	const db = await openDb();
	try {
		await new Promise<void>((resolve, reject) => {
			const tx = db.transaction(STORE, 'readwrite');
			run(tx.objectStore(STORE));
			tx.onerror = () => reject(tx.error ?? new Error('IndexedDB transaction failed'));
			tx.onabort = () => reject(tx.error ?? new Error('IndexedDB transaction aborted'));
			tx.oncomplete = () => resolve();
		});
	} finally {
		db.close();
	}
}

export async function saveSourceImage(record: SourceImageRecord) {
	await withStore('readwrite', (store) => store.put(validateSourceImageRecord(record), SOURCE_KEY));
}

export async function loadSourceImage(): Promise<SourceImageRecord | undefined> {
	const record = await withStore('readonly', (store) => store.get(SOURCE_KEY));
	return record === undefined ? undefined : validateSourceImageRecord(record);
}

export async function saveProcessedImage(record: ProcessedImage) {
	await withStore('readwrite', (store) => store.put(validateProcessedImage(record), PROCESSED_KEY));
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
