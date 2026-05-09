import type { CropRect, ResizeId } from './types';

export const MAX_RESIZE_CACHE_ENTRIES = 2;
export const MAX_RESIZE_CACHE_BYTES = 128 * 1024 * 1024;

type ResizeCacheKeyParts = {
	sourceId: string;
	width: number;
	height: number;
	resize: ResizeId;
	crop?: CropRect;
};

export function resizeCacheKey({ sourceId, width, height, resize, crop }: ResizeCacheKeyParts) {
	const cropKey = crop
		? `${crop.x},${crop.y},${crop.width},${crop.height}`
		: '0,0,full,full';
	return `${sourceId}|${width}x${height}|${resize}|${cropKey}`;
}

export class WorkerResizeCache {
	readonly maxEntries: number;
	readonly maxBytes: number;
	#entries = new Map<string, { image: ImageData; bytes: number }>();
	#bytes = 0;

	constructor(maxEntries = MAX_RESIZE_CACHE_ENTRIES, maxBytes = MAX_RESIZE_CACHE_BYTES) {
		this.maxEntries = maxEntries;
		this.maxBytes = maxBytes;
	}

	get bytes() {
		return this.#bytes;
	}

	get size() {
		return this.#entries.size;
	}

	get(key: string) {
		const entry = this.#entries.get(key);
		if (!entry) return undefined;
		this.#entries.delete(key);
		this.#entries.set(key, entry);
		return entry.image;
	}

	set(key: string, image: ImageData) {
		const bytes = image.width * image.height * 4;
		this.delete(key);
		if (bytes > this.maxBytes || this.maxEntries < 1) return;
		this.#entries.set(key, { image, bytes });
		this.#bytes += bytes;
		this.evictOverflow();
	}

	clear() {
		this.#entries.clear();
		this.#bytes = 0;
	}

	delete(key: string) {
		const entry = this.#entries.get(key);
		if (!entry) return;
		this.#entries.delete(key);
		this.#bytes -= entry.bytes;
	}

	evictOverflow() {
		while (this.#entries.size > this.maxEntries || this.#bytes > this.maxBytes) {
			const oldestKey = this.#entries.keys().next().value as string | undefined;
			if (!oldestKey) return;
			this.delete(oldestKey);
		}
	}
}
