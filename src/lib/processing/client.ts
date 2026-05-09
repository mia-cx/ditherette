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
	selectedPalette,
	sourceImageData,
	sourceMeta
} from '$lib/stores/app';
import { saveProcessedImage } from './db';
import { processingIdentityHash } from './hash';
import { validateWorkerResponse } from './schemas';
import type { ProcessedImage, WorkerRequest } from './types';

let worker: Worker | undefined;
let loadedSourceId: string | undefined;
let activeReject: ((error: Error) => void) | undefined;
let activeRequestId = 0;
let requestId = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let stopAuto: (() => void) | undefined;

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

function processInWorker(): Promise<ProcessedImage> {
	const source = sourceImageData.get();
	if (!source) return Promise.reject(new Error('Upload an image before processing.'));

	supersedeActiveRequest();
	const id = ++requestId;
	activeRequestId = id;
	const activeWorker = getProcessingWorker();
	const sourceId = currentSourceId(source);
	const hash = currentSettingsHash();
	processingProgress.set({ stage: 'Queued', progress: 0 });
	processingError.set(undefined);

	return new Promise((resolve, reject) => {
		activeReject = reject;
		const settle = <T>(callback: (value: T) => void, value: T) => {
			if (activeRequestId !== id) return;
			if (activeReject === reject) activeReject = undefined;
			callback(value);
		};
		const postProcessRequest = () => {
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
			try {
				message = validateWorkerResponse(event.data);
			} catch (error) {
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
			processingProgress.set({ stage: 'Done', progress: 1 });
			settle(resolve, message.image);
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
		if (result.settingsHash !== hash || result.settingsHash !== currentSettingsHash()) return;
		processedImage.set(result);
		processingProgress.set(undefined);
		await saveProcessedImage(result);
	} catch (error) {
		if (error instanceof ProcessingCanceled) return;
		if (hash !== currentSettingsHash()) return;
		const message = error instanceof Error ? error.message : 'Processing failed';
		processingError.set(message);
	}
}

export function scheduleProcessing(delay = 180) {
	if (!sourceImageData.get()) return;
	const hash = currentSettingsHash();
	const previous = processedImage.get();
	if (previous?.settingsHash === hash) return;
	if (timer) clearTimeout(timer);
	timer = setTimeout(() => {
		timer = undefined;
		void processCurrentImage();
	}, delay);
}

export function startAutoProcessing() {
	if (stopAuto) return stopAuto;
	const unsubscribers = [
		sourceImageData.subscribe(() => scheduleProcessing(0)),
		outputSettings.subscribe(() => scheduleProcessing()),
		ditherSettings.subscribe(() => scheduleProcessing()),
		colorSpace.subscribe(() => scheduleProcessing()),
		paletteEnabled.subscribe(() => scheduleProcessing()),
		activePaletteName.subscribe(() => scheduleProcessing()),
		customPalettes.subscribe(() => scheduleProcessing())
	];
	stopAuto = () => {
		for (const unsubscribe of unsubscribers) unsubscribe();
		cancelProcessing();
		stopAuto = undefined;
	};
	return stopAuto;
}
