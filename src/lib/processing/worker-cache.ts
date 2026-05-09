import type { CropRect, ResizeId } from './types';

export const MAX_PIPELINE_CACHE_BRANCHES = 3;
export const MAX_PIPELINE_CACHE_BYTES = 256 * 1024 * 1024;
export const MAX_PALETTE_VECTOR_CACHE_ENTRIES = 16;
export const IDENTITY_GRADE_KEY = 'identity';

type PipelineBranchKeyParts = {
	sourceId: string;
	width: number;
	height: number;
	resize: ResizeId;
	crop?: CropRect;
	gradeKey?: string;
};

type PipelineBranch = {
	resized: ImageData;
	bytes: number;
	colorMappings: Map<string, { value: unknown; bytes: number }>;
};

export function pipelineBranchKey({
	sourceId,
	width,
	height,
	resize,
	crop,
	gradeKey = IDENTITY_GRADE_KEY
}: PipelineBranchKeyParts) {
	const cropKey = crop ? `${crop.x},${crop.y},${crop.width},${crop.height}` : '0,0,full,full';
	return `${sourceId}|${width}x${height}|${resize}|${cropKey}|grade:${gradeKey}`;
}

export function resizeCacheKey(parts: PipelineBranchKeyParts) {
	return pipelineBranchKey(parts);
}

export class PipelineBranchCache {
	readonly maxBranches: number;
	readonly maxBytes: number;
	#branches = new Map<string, PipelineBranch>();
	#bytes = 0;

	constructor(maxBranches = MAX_PIPELINE_CACHE_BRANCHES, maxBytes = MAX_PIPELINE_CACHE_BYTES) {
		this.maxBranches = maxBranches;
		this.maxBytes = maxBytes;
	}

	get bytes() {
		return this.#bytes;
	}

	get size() {
		return this.#branches.size;
	}

	getResized(key: string) {
		const branch = this.#branches.get(key);
		if (!branch) return undefined;
		this.touch(key, branch);
		return branch.resized;
	}

	setResized(key: string, resized: ImageData) {
		const bytes = resized.width * resized.height * 4;
		this.delete(key);
		if (bytes > this.maxBytes || this.maxBranches < 1) return;
		this.#branches.set(key, { resized, bytes, colorMappings: new Map() });
		this.#bytes += bytes;
		this.evictOverflow();
	}

	getColorMapping<T>(branchKey: string, mappingKey: string): T | undefined {
		const branch = this.#branches.get(branchKey);
		if (!branch) return undefined;
		const mapping = branch.colorMappings.get(mappingKey);
		if (!mapping) return undefined;
		this.touch(branchKey, branch);
		return mapping.value as T;
	}

	canStoreColorMapping(branchKey: string, bytes: number) {
		const branch = this.#branches.get(branchKey);
		if (!branch) return false;
		return bytes <= this.maxBytes && branch.bytes + bytes <= this.maxBytes;
	}

	setColorMapping(branchKey: string, mappingKey: string, value: unknown, bytes: number) {
		const branch = this.#branches.get(branchKey);
		if (!branch || bytes > this.maxBytes) return false;
		const previous = branch.colorMappings.get(mappingKey);
		const nextBranchBytes = branch.bytes - (previous?.bytes ?? 0) + bytes;
		if (nextBranchBytes > this.maxBytes) return false;
		if (previous) {
			branch.bytes -= previous.bytes;
			this.#bytes -= previous.bytes;
		}
		branch.colorMappings.set(mappingKey, { value, bytes });
		branch.bytes += bytes;
		this.#bytes += bytes;
		this.touch(branchKey, branch);
		this.evictOverflow();
		return this.#branches.get(branchKey)?.colorMappings.has(mappingKey) ?? false;
	}

	clear() {
		this.#branches.clear();
		this.#bytes = 0;
	}

	delete(key: string) {
		const branch = this.#branches.get(key);
		if (!branch) return;
		this.#branches.delete(key);
		this.#bytes -= branch.bytes;
	}

	has(key: string) {
		return this.#branches.has(key);
	}

	keys() {
		return [...this.#branches.keys()];
	}

	private touch(key: string, branch: PipelineBranch) {
		this.#branches.delete(key);
		this.#branches.set(key, branch);
	}

	private evictOverflow() {
		while (this.#branches.size > this.maxBranches) {
			const oldestKey = this.#branches.keys().next().value as string | undefined;
			if (!oldestKey) return;
			this.delete(oldestKey);
		}

		while (this.#bytes > this.maxBytes && this.deleteOldestColorMapping()) {
			// Prefer dropping derived data before dropping resized branches.
		}

		while (this.#bytes > this.maxBytes) {
			const oldestKey = this.#branches.keys().next().value as string | undefined;
			if (!oldestKey) return;
			this.delete(oldestKey);
		}
	}

	private deleteOldestColorMapping() {
		for (const branch of this.#branches.values()) {
			const oldestMappingKey = branch.colorMappings.keys().next().value as string | undefined;
			if (!oldestMappingKey) continue;
			const mapping = branch.colorMappings.get(oldestMappingKey);
			if (!mapping) continue;
			branch.colorMappings.delete(oldestMappingKey);
			branch.bytes -= mapping.bytes;
			this.#bytes -= mapping.bytes;
			return true;
		}
		return false;
	}
}

export class PaletteVectorCache<T> {
	readonly maxEntries: number;
	#entries = new Map<string, T>();

	constructor(maxEntries = MAX_PALETTE_VECTOR_CACHE_ENTRIES) {
		this.maxEntries = maxEntries;
	}

	get size() {
		return this.#entries.size;
	}

	get(key: string) {
		const value = this.#entries.get(key);
		if (!value) return undefined;
		this.#entries.delete(key);
		this.#entries.set(key, value);
		return value;
	}

	set(key: string, value: T) {
		if (this.maxEntries < 1) return;
		this.#entries.delete(key);
		this.#entries.set(key, value);
		while (this.#entries.size > this.maxEntries) {
			const oldestKey = this.#entries.keys().next().value as string | undefined;
			if (!oldestKey) return;
			this.#entries.delete(oldestKey);
		}
	}

	clear() {
		this.#entries.clear();
	}
}
