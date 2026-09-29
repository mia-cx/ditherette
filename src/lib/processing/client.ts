import { Effect } from 'effect';
import {
	activePalette,
	activePaletteName,
	colorSpace,
	customPalettes,
	ditherSettings,
	effectsTable,
	outputPreview,
	outputSettings,
	paletteEnabled,
	processedImage,
	processingError,
	processingProgress,
	recordProcessingMetrics,
	selectedPalette,
	sourceImageData,
	sourceMeta
} from '$lib/stores/app';
import { activeEffectSteps } from '$lib/stores/effects';
import { saveProcessedImage } from './db';
import { processingIdentityHash } from './hash';
import { validateWorkerResponse } from './schemas';
import type { ProcessingMetricsSample, ProcessingStageTiming } from './metrics';
import type { DitherSettings, OutputSettings, ProcessedImage, WorkerRequest } from './types';

let worker: Worker | undefined;
let loadedSourceId: string | undefined;
let activeReject: ((error: Error) => void) | undefined;
let activeRequestId = 0;
let activeStartedAt = 0;
let requestId = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let stopAuto: (() => void) | undefined;
let workerNeedsReplacement = false;
/** A superseded job may still be running on the kept worker, so the next request may wait. */
let workerMayBeBusy = false;
let persistTimer: ReturnType<typeof setTimeout> | undefined;

const SLIDER_DEBOUNCE_MS = 180;
/**
 * A superseded job younger than this finishes on its worker, so the next job keeps the loaded
 * source and the package's stage cache. Older jobs may run long, so their worker is replaced, and
 * a kept worker that still hasn't reached the next job after this long is replaced too.
 */
const REPLACE_BUSY_WORKER_AFTER_MS = 500;
/** Saving clones every index into IndexedDB, so only the result edits settle on is saved. */
const PERSIST_DELAY_MS = 1000;
const OUTPUT_SLIDER_FIELDS = new Set<keyof OutputSettings>([
	'width',
	'height',
	'scaleFactor',
	'alphaThreshold'
]);
const DITHER_SLIDER_FIELDS = new Set<keyof DitherSettings>([
	'strength',
	'placementRadius',
	'placementThreshold',
	'placementSoftness'
]);

class ProcessingCanceled extends Error {
	constructor(message = 'Processing was cancelled.') {
		super(message);
		this.name = 'ProcessingCanceled';
	}
}

export function cancelProcessing() {
	if (timer) clearTimeout(timer);
	timer = undefined;
	// A new source cancels processing first; an old result saved after that would outlive it.
	if (persistTimer) clearTimeout(persistTimer);
	persistTimer = undefined;
	activeRequestId = ++requestId;
	activeReject?.(new ProcessingCanceled());
	activeReject = undefined;
	terminateProcessingWorker();
	processingProgress.set(undefined);
}

function terminateProcessingWorker() {
	workerMayBeBusy = false;
	worker?.terminate();
	worker = undefined;
	loadedSourceId = undefined;
	workerNeedsReplacement = false;
}

function getProcessingWorker() {
	if (worker) return worker;
	worker = new Worker(new URL('../workers/processor.worker.ts', import.meta.url), {
		type: 'module'
	});
	loadedSourceId = undefined;
	return worker;
}

function resetProcessingWorker(activeWorker: Worker) {
	if (worker !== activeWorker) return;
	terminateProcessingWorker();
}

function supersedeActiveRequest() {
	const superseded = activeRequestId;
	activeRequestId = ++requestId;
	if (activeReject) {
		if (performance.now() - activeStartedAt > REPLACE_BUSY_WORKER_AFTER_MS)
			workerNeedsReplacement = true;
		else {
			worker?.postMessage({ id: superseded, type: 'cancel' } satisfies WorkerRequest);
			workerMayBeBusy = true;
		}
	}
	activeReject?.(new ProcessingCanceled('Processing was superseded by newer settings.'));
	activeReject = undefined;
}

function currentSourceId(image: ImageData) {
	const meta = sourceMeta.get();
	if (!meta) return `memory:${image.width}x${image.height}`;
	return `${meta.name}:${meta.width}x${meta.height}:${meta.type}:${meta.updatedAt}`;
}

export function currentSettingsHash() {
	const palette = activePalette.get();
	return processingIdentityHash({
		output: outputSettings.get(),
		dither: ditherSettings.get(),
		colorSpace: colorSpace.get(),
		effects: activeEffectSteps.get(),
		paletteName: palette.name,
		paletteSource: palette.source,
		palette: selectedPalette.get(),
		source: sourceMeta.get()
	});
}

type ProcessInWorkerResult = {
	image: ProcessedImage;
	preview?: ImageBitmap[];
	metrics?: ProcessingMetricsSample;
};

type ProcessingSchedule = {
	scheduledAt: number;
	scheduledDelay: number;
};

function processInWorker(schedule?: ProcessingSchedule): Promise<ProcessInWorkerResult> {
	const source = sourceImageData.get();
	if (!source) return Promise.reject(new Error('Upload an image before processing.'));

	supersedeActiveRequest();
	if (workerNeedsReplacement) terminateProcessingWorker();
	const id = ++requestId;
	activeRequestId = id;
	activeStartedAt = performance.now();
	const activeWorker = getProcessingWorker();
	const sourceId = currentSourceId(source);
	const hash = currentSettingsHash();
	const processStartedAt = performance.now();
	const mainTimings: ProcessingStageTiming[] = [];
	const scheduledAt = schedule?.scheduledAt;
	const metricsStartedAt = scheduledAt ?? processStartedAt;
	const debounceMs = scheduledAt === undefined ? 0 : Math.max(0, processStartedAt - scheduledAt);
	mainTimings.push({ name: 'main debounce wait', ms: debounceMs });
	mainTimings.push({ name: 'main scheduled delay', ms: schedule?.scheduledDelay ?? 0 });
	processingProgress.set({ stage: 'Queued', progress: 0 });
	processingError.set(undefined);

	return new Promise((resolve, reject) => {
		activeReject = reject;
		// `process` runs synchronously, so a kept worker still on superseded work cannot even read
		// this request. If it stays silent, replace it and start over on a fresh worker.
		const watchdog = workerMayBeBusy
			? setTimeout(() => {
					if (activeReject !== reject) return;
					activeReject = undefined;
					terminateProcessingWorker();
					processInWorker(schedule).then(resolve, reject);
				}, REPLACE_BUSY_WORKER_AFTER_MS)
			: undefined;
		const isCurrent = () =>
			worker === activeWorker && activeRequestId === id && activeReject === reject;
		const settle = <T>(callback: (value: T) => void, value: T) => {
			if (activeRequestId !== id) return;
			if (activeReject === reject) activeReject = undefined;
			callback(value);
		};
		let sourceLoadPostedAt = 0;
		let processPostedAt = 0;
		let responseValidationMs = 0;
		const postProcessRequest = () => {
			processPostedAt = performance.now();
			activeWorker.postMessage({
				id,
				type: 'process',
				sourceId,
				settings: {
					output: outputSettings.get(),
					dither: ditherSettings.get(),
					colorSpace: colorSpace.get(),
					effects: activeEffectSteps.get()
				},
				palette: selectedPalette.get(),
				settingsHash: hash
			} satisfies WorkerRequest);
		};

		activeWorker.onmessage = (event: MessageEvent<unknown>) => {
			if (!isCurrent()) return;
			// A reused worker may still deliver an older response. Reject its identity before validation.
			if (
				event.data &&
				typeof event.data === 'object' &&
				'id' in event.data &&
				Number.isInteger(event.data.id) &&
				event.data.id !== id
			)
				return;
			let message;
			const validationStart = performance.now();
			try {
				message = validateWorkerResponse(event.data);
				responseValidationMs += performance.now() - validationStart;
			} catch (error) {
				responseValidationMs += performance.now() - validationStart;
				processingProgress.set(undefined);
				settle(reject, error instanceof Error ? error : new Error('Worker response was invalid.'));
				return;
			}
			if (message.id !== id || activeRequestId !== id) return;
			clearTimeout(watchdog);
			workerMayBeBusy = false;
			if (message.type === 'progress') {
				processingProgress.set({
					stage: message.stage,
					progress: message.progress,
					completed: message.completed,
					total: message.total
				});
				return;
			}
			if (message.type === 'effects-table') {
				effectsTable.set(message.table);
				return;
			}
			if (message.type === 'source-loaded') {
				if (sourceLoadPostedAt) {
					mainTimings.push({
						name: 'main source load round trip',
						ms: performance.now() - sourceLoadPostedAt
					});
				}
				if (message.sourceId !== sourceId) {
					processingProgress.set(undefined);
					settle(reject, new Error('Worker loaded the wrong source image.'));
					return;
				}
				loadedSourceId = sourceId;
				postProcessRequest();
				return;
			}
			if (message.type === 'error') {
				processingProgress.set(undefined);
				// A fresh worker also clears rejected module imports and Wasm compilation promises.
				if (message.restartWorker) resetProcessingWorker(activeWorker);
				settle(reject, new Error(message.message));
				return;
			}
			const completedAt = performance.now();
			if (processPostedAt) {
				mainTimings.push({
					name: 'main worker round trip',
					ms: completedAt - processPostedAt
				});
			}
			mainTimings.push({ name: 'main response validation', ms: responseValidationMs });
			processingProgress.set({ stage: 'Done', progress: 1 });
			settle(resolve, {
				image: message.image,
				preview: message.preview,
				metrics: message.metrics
					? {
							...message.metrics,
							startedAt: metricsStartedAt,
							completedAt,
							totalMs: completedAt - metricsStartedAt,
							timings: [...mainTimings, ...message.metrics.timings]
						}
					: undefined
			});
		};
		activeWorker.onerror = () => {
			if (!isCurrent()) return;
			processingProgress.set(undefined);
			resetProcessingWorker(activeWorker);
			settle(reject, new Error('Worker crashed while processing the image.'));
		};

		if (loadedSourceId === sourceId) {
			postProcessRequest();
			return;
		}

		// The worker builds a new table for a new source; the old one maps other colours.
		effectsTable.set(undefined);
		processingProgress.set({ stage: 'Loading source', progress: 0.02 });
		sourceLoadPostedAt = performance.now();
		activeWorker.postMessage({ id, type: 'load-source', sourceId, source } satisfies WorkerRequest);
	});
}

export async function processCurrentImage(schedule?: ProcessingSchedule) {
	const hash = currentSettingsHash();
	const previous = processedImage.get();
	if (previous?.settingsHash === hash) return;
	// Keep the last valid output on screen while non-crop edits reprocess.
	// Source and crop changes clear it at their boundaries because the old frame shape is misleading there.
	const program = Effect.tryPromise({
		try: () => processInWorker(schedule),
		catch: (error) => (error instanceof Error ? error : new Error('Processing failed'))
	});

	try {
		const result = await Effect.runPromise(program);
		if (result.image.settingsHash !== hash || result.image.settingsHash !== currentSettingsHash()) {
			result.preview?.forEach((level) => level.close());
			return;
		}
		// Bitmaps hold their pixels until closed, so free the levels this output replaces.
		outputPreview.get()?.levels.forEach((level) => level.close());
		outputPreview.set(result.preview && { image: result.image, levels: result.preview });
		processedImage.set(result.image);
		processingProgress.set(undefined);
		persistWhenSettled();
		if (result.metrics) recordProcessingMetrics(result.metrics);
	} catch (error) {
		if (error instanceof ProcessingCanceled) return;
		if (hash !== currentSettingsHash()) return;
		const message = error instanceof Error ? error.message : 'Processing failed';
		processingError.set(message);
	}
}

/** Save the processed image once no newer result replaces it for a moment. */
function persistWhenSettled() {
	if (persistTimer) clearTimeout(persistTimer);
	persistTimer = setTimeout(() => {
		persistTimer = undefined;
		const image = processedImage.get();
		if (!image) return;
		saveProcessedImage(image).catch((error: unknown) => {
			if (processedImage.get() !== image) return;
			processingError.set(error instanceof Error ? error.message : 'Saving the output failed.');
		});
	}, PERSIST_DELAY_MS);
}

export function scheduleProcessing(delay = 0) {
	if (!sourceImageData.get()) return;
	const hash = currentSettingsHash();
	const previous = processedImage.get();
	if (timer) clearTimeout(timer);
	timer = undefined;
	supersedeActiveRequest();
	processingProgress.set(undefined);
	if (previous?.settingsHash === hash) {
		if (workerNeedsReplacement) terminateProcessingWorker();
		return;
	}
	const schedule: ProcessingSchedule = {
		scheduledAt: performance.now(),
		scheduledDelay: delay
	};
	timer = setTimeout(() => {
		timer = undefined;
		void processCurrentImage(schedule);
	}, delay);
}

function changedKeys<T extends object>(previous: T, next: T) {
	return (Object.keys(next) as Array<keyof T>).filter((key) => previous[key] !== next[key]);
}

function onlySliderFieldsChanged<T extends object>(
	previous: T,
	next: T,
	sliderFields: ReadonlySet<keyof T>
) {
	const keys = changedKeys(previous, next);
	return keys.length > 0 && keys.every((key) => sliderFields.has(key));
}

export function outputProcessingDelay(previous: OutputSettings, next: OutputSettings) {
	return onlySliderFieldsChanged(previous, next, OUTPUT_SLIDER_FIELDS) ? SLIDER_DEBOUNCE_MS : 0;
}

export function ditherProcessingDelay(previous: DitherSettings, next: DitherSettings) {
	return onlySliderFieldsChanged(previous, next, DITHER_SLIDER_FIELDS) ? SLIDER_DEBOUNCE_MS : 0;
}

export function startAutoProcessing() {
	if (stopAuto) return stopAuto;
	let previousOutputSettings = outputSettings.get();
	let previousDitherSettings = ditherSettings.get();
	let previousEffectSteps = JSON.stringify(activeEffectSteps.get());
	const unsubscribers = [
		sourceImageData.subscribe(() => scheduleProcessing(0)),
		outputSettings.subscribe((settings) => {
			const delay = outputProcessingDelay(previousOutputSettings, settings);
			previousOutputSettings = settings;
			scheduleProcessing(delay);
		}),
		ditherSettings.subscribe((settings) => {
			const delay = ditherProcessingDelay(previousDitherSettings, settings);
			previousDitherSettings = settings;
			scheduleProcessing(delay);
		}),
		colorSpace.subscribe(() => scheduleProcessing(0)),
		activeEffectSteps.listen((steps) => {
			// Renaming or reordering disabled layers leaves the steps unchanged.
			const next = JSON.stringify(steps);
			if (next === previousEffectSteps) return;
			previousEffectSteps = next;
			scheduleProcessing(SLIDER_DEBOUNCE_MS);
		}),
		paletteEnabled.subscribe(() => scheduleProcessing(0)),
		activePaletteName.subscribe(() => scheduleProcessing(0)),
		customPalettes.subscribe(() => scheduleProcessing(0))
	];
	stopAuto = () => {
		for (const unsubscribe of unsubscribers) unsubscribe();
		cancelProcessing();
		stopAuto = undefined;
	};
	return stopAuto;
}
