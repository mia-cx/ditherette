import { Effect } from 'effect';
import {
	activePalette,
	activePaletteName,
	colorSpace,
	customPalettes,
	ditherSettings,
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
import { saveProcessedImage } from './db';
import { processingIdentityHash } from './hash';
import { validateWorkerResponse } from './schemas';
import type { ProcessingMetricsSample, ProcessingStageTiming } from './metrics';
import type { DitherSettings, OutputSettings, ProcessedImage, WorkerRequest } from './types';

let worker: Worker | undefined;
let loadedSourceId: string | undefined;
let activeReject: ((error: Error) => void) | undefined;
let activeRequestId = 0;
let requestId = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let stopAuto: (() => void) | undefined;
let scheduledAt = 0;
let scheduledDelay = 0;

const SLIDER_DEBOUNCE_MS = 180;
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
	constructor(message = 'Processing was canceled.') {
		super(message);
		this.name = 'ProcessingCanceled';
	}
}

export function cancelProcessing() {
	if (timer) clearTimeout(timer);
	timer = undefined;
	activeRequestId = ++requestId;
	activeReject?.(new ProcessingCanceled());
	activeReject = undefined;
	worker?.terminate();
	worker = undefined;
	loadedSourceId = undefined;
	processingProgress.set(undefined);
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
	worker = undefined;
	loadedSourceId = undefined;
}

function supersedeActiveRequest() {
	const canceledId = activeRequestId;
	activeReject?.(new ProcessingCanceled('Processing was superseded by newer settings.'));
	activeReject = undefined;
	if (worker && canceledId > 0) {
		worker.postMessage({ id: canceledId, type: 'cancel' } satisfies WorkerRequest);
	}
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
		paletteName: palette.name,
		paletteSource: palette.source,
		palette: selectedPalette.get(),
		source: sourceMeta.get()
	});
}

type ProcessInWorkerResult = {
	image: ProcessedImage;
	metrics?: ProcessingMetricsSample;
};

function processInWorker(): Promise<ProcessInWorkerResult> {
	const source = sourceImageData.get();
	if (!source) return Promise.reject(new Error('Upload an image before processing.'));

	supersedeActiveRequest();
	const id = ++requestId;
	activeRequestId = id;
	const activeWorker = getProcessingWorker();
	const sourceId = currentSourceId(source);
	const hash = currentSettingsHash();
	const processStartedAt = performance.now();
	const mainTimings: ProcessingStageTiming[] = [];
	const debounceMs = scheduledAt ? Math.max(0, processStartedAt - scheduledAt) : 0;
	mainTimings.push({ name: 'main debounce wait', ms: debounceMs });
	mainTimings.push({ name: 'main scheduled delay', ms: scheduledDelay });
	processingProgress.set({ stage: 'Queued', progress: 0 });
	processingError.set(undefined);

	return new Promise((resolve, reject) => {
		activeReject = reject;
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
					colorSpace: colorSpace.get()
				},
				palette: selectedPalette.get(),
				settingsHash: hash
			} satisfies WorkerRequest);
		};

		activeWorker.onmessage = (event: MessageEvent<unknown>) => {
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
			if (message.type === 'progress') {
				processingProgress.set({ stage: message.stage, progress: message.progress });
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
				metrics: message.metrics
					? {
							...message.metrics,
							startedAt: scheduledAt || processStartedAt,
							completedAt,
							totalMs: completedAt - (scheduledAt || processStartedAt),
							timings: [...mainTimings, ...message.metrics.timings]
						}
					: undefined
			});
		};
		activeWorker.onerror = () => {
			processingProgress.set(undefined);
			resetProcessingWorker(activeWorker);
			settle(reject, new Error('Worker crashed while processing the image.'));
		};

		if (loadedSourceId === sourceId) {
			postProcessRequest();
			return;
		}

		processingProgress.set({ stage: 'Loading source', progress: 0.02 });
		sourceLoadPostedAt = performance.now();
		activeWorker.postMessage({ id, type: 'load-source', sourceId, source } satisfies WorkerRequest);
	});
}

export async function processCurrentImage() {
	const hash = currentSettingsHash();
	const previous = processedImage.get();
	if (previous?.settingsHash === hash) return;
	// Keep the last valid output on screen while non-crop edits reprocess.
	// Source and crop changes clear it at their boundaries because the old frame shape is misleading there.
	const program = Effect.tryPromise({
		try: processInWorker,
		catch: (error) => (error instanceof Error ? error : new Error('Processing failed'))
	});

	try {
		const result = await Effect.runPromise(program);
		if (result.image.settingsHash !== hash || result.image.settingsHash !== currentSettingsHash())
			return;
		processedImage.set(result.image);
		processingProgress.set(undefined);
		const persistStart = performance.now();
		await saveProcessedImage(result.image);
		if (result.metrics) {
			const completedAt = performance.now();
			recordProcessingMetrics({
				...result.metrics,
				completedAt,
				totalMs: completedAt - result.metrics.startedAt,
				timings: [
					...result.metrics.timings,
					{ name: 'main persist processed image', ms: completedAt - persistStart }
				]
			});
		}
	} catch (error) {
		if (error instanceof ProcessingCanceled) return;
		if (hash !== currentSettingsHash()) return;
		const message = error instanceof Error ? error.message : 'Processing failed';
		processingError.set(message);
	}
}

export function scheduleProcessing(delay = 0) {
	if (!sourceImageData.get()) return;
	const hash = currentSettingsHash();
	const previous = processedImage.get();
	if (previous?.settingsHash === hash) return;
	if (timer) clearTimeout(timer);
	scheduledAt = performance.now();
	scheduledDelay = delay;
	timer = setTimeout(() => {
		timer = undefined;
		void processCurrentImage();
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
