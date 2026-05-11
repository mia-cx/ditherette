import { bayerSizeForAlgorithm } from './bayer';
import {
	prepareQuantize,
	usesAdaptivePlacement,
	type QuantizeCaches,
	type QuantizeCounterName,
	type QuantizeResult
} from './quantize-shared';
import type { EnabledPaletteColor, ProcessingSettings } from './types';

type RowQuantizeDone = {
	id: number;
	type: 'done';
	startY: number;
	endY: number;
	indices?: Uint8Array;
	counts: Record<string, number>;
};

type RowQuantizeFailure = {
	id: number;
	type: 'error';
	message: string;
};

type RowQuantizeResponse = RowQuantizeDone | RowQuantizeFailure;

type RowQuantizeJob = {
	id: number;
	width: number;
	height: number;
	startY: number;
	endY: number;
	settings: ProcessingSettings;
	palette: EnabledPaletteColor[];
	sourceBuffer?: SharedArrayBuffer;
	sourceRows?: Uint8ClampedArray;
	outputBuffer?: SharedArrayBuffer;
	cancelBuffer?: SharedArrayBuffer;
	randomStepStart?: number;
};

type RowWorker = Worker & { busy?: boolean };

type RowWorkerOptions = {
	shouldCancel?: () => boolean;
	workerCount?: number;
	minPixels?: number;
};

const DEFAULT_MIN_PIXELS = 1_000_000;
const MAX_WORKERS = 8;
const ROW_WORKER_URL = new URL('./quantize-row.worker.ts', import.meta.url);

export function canUseRowWorkerQuantize(
	settings: ProcessingSettings,
	pixels: number,
	minPixels = DEFAULT_MIN_PIXELS
) {
	if (pixels < minPixels) return false;
	if (typeof Worker === 'undefined') return false;
	if (usesAdaptivePlacement(settings)) return false;
	if (settings.dither.algorithm === 'none' || settings.dither.algorithm === 'random') return true;
	return Boolean(bayerSizeForAlgorithm(settings.dither.algorithm));
}

export async function quantizeImageWithRowWorkers(
	image: ImageData,
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings,
	caches?: QuantizeCaches,
	options: RowWorkerOptions = {}
): Promise<QuantizeResult | undefined> {
	const pixels = image.width * image.height;
	if (!canUseRowWorkerQuantize(settings, pixels, options.minPixels)) return undefined;

	const prepared = prepareQuantize(palette, settings, caches);
	if ('indices' in prepared) {
		return {
			...prepared,
			indices: new Uint8Array(pixels).fill(prepared.transparentIndex)
		};
	}
	if (options.shouldCancel?.()) throw new Error('Processing was canceled.');

	const workerCount = Math.max(
		1,
		Math.min(options.workerCount ?? defaultWorkerCount(), image.height, MAX_WORKERS)
	);
	if (workerCount <= 1) return undefined;

	const useSharedBuffers = typeof SharedArrayBuffer !== 'undefined';
	const rowsPerWorker = Math.ceil(image.height / workerCount);
	const sourceBuffer = useSharedBuffers ? new SharedArrayBuffer(image.data.byteLength) : undefined;
	const outputBuffer = useSharedBuffers ? new SharedArrayBuffer(pixels) : undefined;
	const cancelBuffer = useSharedBuffers
		? new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT)
		: undefined;
	if (sourceBuffer) new Uint8ClampedArray(sourceBuffer).set(image.data);
	const cancelFlag = cancelBuffer ? new Int32Array(cancelBuffer) : undefined;
	const workers = createPool(workerCount);
	let nextJobId = 1;
	const startRows = Array.from({ length: workerCount }, (_, ordinal) => ordinal * rowsPerWorker);
	const randomStepStarts = randomStepStartsForRows(image, startRows, settings, prepared.strength);
	const counts: Record<string, number> = {};
	let canceled = false;
	let cancelTimer: ReturnType<typeof setInterval> | undefined;
	const cancelPromise = new Promise<never>((_, reject) => {
		cancelTimer = setInterval(() => {
			if (!options.shouldCancel?.() || canceled) return;
			canceled = true;
			if (cancelFlag) Atomics.store(cancelFlag, 0, 1);
			terminatePool(workers);
			reject(new Error('Processing was canceled.'));
		}, 8);
	});

	try {
		const jobs = workers.map((worker, ordinal) => {
			const startY = startRows[ordinal]!;
			const endY = Math.min(image.height, startY + rowsPerWorker);
			return runWorkerJob(worker, {
				id: nextJobId++,
				width: image.width,
				height: image.height,
				startY,
				endY,
				settings,
				palette: prepared.nextPalette,
				sourceBuffer,
				sourceRows: sourceBuffer ? undefined : rowsForImage(image, startY, endY),
				outputBuffer,
				cancelBuffer,
				randomStepStart: randomStepStarts?.[ordinal]
			});
		});
		const results = await Promise.race([Promise.all(jobs), cancelPromise]);
		if (canceled || options.shouldCancel?.()) throw new Error('Processing was canceled.');
		const indices = outputBuffer ? new Uint8Array(outputBuffer).slice() : new Uint8Array(pixels);
		for (const result of results) {
			mergeCounts(counts, result.counts);
			if (!outputBuffer && result.indices) indices.set(result.indices, result.startY * image.width);
		}
		for (const [name, amount] of Object.entries(counts))
			caches?.recordCount?.(name as QuantizeCounterName, amount);
		return {
			indices,
			palette: prepared.nextPalette,
			transparentIndex: prepared.transparentIndexValue,
			warnings: prepared.warnings
		};
	} finally {
		if (cancelTimer) clearInterval(cancelTimer);
		terminatePool(workers);
	}
}

function createPool(count: number): RowWorker[] {
	return Array.from(
		{ length: count },
		() => new Worker(ROW_WORKER_URL, { type: 'module' }) as RowWorker
	);
}

function terminatePool(workers: RowWorker[]) {
	for (const worker of workers) worker.terminate();
}

function runWorkerJob(worker: RowWorker, job: RowQuantizeJob): Promise<RowQuantizeDone> {
	worker.busy = true;
	return new Promise((resolve, reject) => {
		const cleanup = () => {
			worker.busy = false;
			worker.removeEventListener('message', onMessage);
			worker.removeEventListener('error', onError);
		};
		const onMessage = (event: MessageEvent<RowQuantizeResponse>) => {
			if (event.data.id !== job.id) return;
			cleanup();
			if (event.data.type === 'error') {
				reject(new Error(event.data.message));
				return;
			}
			resolve(event.data);
		};
		const onError = (event: ErrorEvent) => {
			cleanup();
			reject(event.error instanceof Error ? event.error : new Error(event.message));
		};
		worker.addEventListener('message', onMessage);
		worker.addEventListener('error', onError);
		worker.postMessage(job);
	});
}

function rowsForImage(image: ImageData, startY: number, endY: number) {
	return image.data.slice(startY * image.width * 4, endY * image.width * 4);
}

function randomStepStartsForRows(
	image: ImageData,
	startRows: number[],
	settings: ProcessingSettings,
	strength: number
) {
	if (settings.dither.algorithm !== 'random' || strength <= 0) return undefined;
	if (settings.output.alphaMode !== 'preserve')
		return startRows.map((startY) => startY * image.width);

	const starts = new Array<number>(startRows.length);
	let nextStart = 0;
	let steps = 0;
	const alphaThreshold = settings.output.alphaThreshold;
	for (let y = 0; y < image.height; y++) {
		while (nextStart < startRows.length && startRows[nextStart] === y) {
			starts[nextStart] = steps;
			nextStart++;
		}
		const rowEnd = (y + 1) * image.width * 4;
		for (let offset = y * image.width * 4 + 3; offset < rowEnd; offset += 4) {
			if (image.data[offset]! > alphaThreshold) steps++;
		}
	}
	while (nextStart < startRows.length) starts[nextStart++] = steps;
	return starts;
}

function mergeCounts(target: Record<string, number>, source: Record<string, number>) {
	for (const [name, amount] of Object.entries(source)) target[name] = (target[name] ?? 0) + amount;
}

function defaultWorkerCount() {
	const navigatorLike = globalThis.navigator as Navigator | undefined;
	return Math.max(2, Math.min(navigatorLike?.hardwareConcurrency ?? 4, MAX_WORKERS));
}
