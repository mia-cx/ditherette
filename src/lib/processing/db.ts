import { validateSourceImageSize } from './types';
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
			const request = run(tx.objectStore(STORE));
			request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'));
			request.onsuccess = () => resolve(request.result);
			tx.onerror = () => reject(tx.error ?? new Error('IndexedDB transaction failed'));
		});
	} finally {
		db.close();
	}
}

export async function saveSourceImage(record: SourceImageRecord) {
	await withStore('readwrite', (store) => store.put(record, SOURCE_KEY));
}

export async function loadSourceImage(): Promise<SourceImageRecord | undefined> {
	return (await withStore('readonly', (store) => store.get(SOURCE_KEY))) as
		| SourceImageRecord
		| undefined;
}

export async function saveProcessedImage(record: ProcessedImage) {
	await withStore('readwrite', (store) => store.put(record, PROCESSED_KEY));
}

export async function loadProcessedImage(): Promise<ProcessedImage | undefined> {
	return (await withStore('readonly', (store) => store.get(PROCESSED_KEY))) as
		| ProcessedImage
		| undefined;
}

export async function clearPersistedImages() {
	await Promise.all([
		withStore('readwrite', (store) => store.delete(SOURCE_KEY)),
		withStore('readwrite', (store) => store.delete(PROCESSED_KEY))
	]);
}

export async function decodeBlob(
	blob: Blob
): Promise<{ imageData: ImageData; width: number; height: number }> {
	const bitmap = await createImageBitmap(blob);
	try {
		validateSourceImageSize(bitmap.width, bitmap.height);
		const canvas =
			typeof OffscreenCanvas !== 'undefined'
				? new OffscreenCanvas(bitmap.width, bitmap.height)
				: Object.assign(document.createElement('canvas'), {
						width: bitmap.width,
						height: bitmap.height
					});
		const context = canvas.getContext('2d', { willReadFrequently: true });
		if (!context) throw new Error('Canvas 2D is not available');
		context.drawImage(bitmap, 0, 0);
		return {
			imageData: context.getImageData(0, 0, bitmap.width, bitmap.height),
			width: bitmap.width,
			height: bitmap.height
		};
	} finally {
		bitmap.close();
	}
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

export function settingsHash(value: unknown): string {
	return JSON.stringify(value);
}

export function browserCanProcessImages() {
	return typeof indexedDB !== 'undefined' && typeof createImageBitmap !== 'undefined';
}
