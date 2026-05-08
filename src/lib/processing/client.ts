import { Effect } from 'effect';
import {
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
import { settingsHash } from './hash';
import { validateWorkerResponse } from './schemas';
import type { ProcessedImage } from './types';

let worker: Worker | undefined;
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
	processingProgress.set(undefined);
}

function openProcessingWorker() {
	activeReject?.(new ProcessingCanceled('Processing was superseded by newer settings.'));
	activeReject = undefined;
	worker?.terminate();
	worker = new Worker(new URL('../workers/processor.worker.ts', import.meta.url), {
		type: 'module'
	});
	return worker;
}

function closeProcessingWorker(activeWorker: Worker) {
	if (worker === activeWorker) worker = undefined;
	activeWorker.terminate();
}

export function currentSettingsHash() {
	return settingsHash({
		output: outputSettings.get(),
		dither: ditherSettings.get(),
		colorSpace: colorSpace.get(),
		paletteName: activePaletteName.get(),
		customPalettes: customPalettes.get(),
		palette: paletteEnabled.get(),
		source: sourceMeta.get()
	});
}

function processInWorker(): Promise<ProcessedImage> {
	const source = sourceImageData.get();
	if (!source) return Promise.reject(new Error('Upload an image before processing.'));

	const id = ++requestId;
	activeRequestId = id;
	const activeWorker = openProcessingWorker();
	const hash = currentSettingsHash();
	processingProgress.set({ stage: 'Queued', progress: 0 });
	processingError.set(undefined);

	return new Promise((resolve, reject) => {
		activeReject = reject;
		const settle = <T>(callback: (value: T) => void, value: T) => {
			if (activeRequestId !== id) return;
			if (activeReject === reject) activeReject = undefined;
			closeProcessingWorker(activeWorker);
			callback(value);
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
			settle(reject, new Error('Worker crashed while processing the image.'));
		};
		activeWorker.postMessage({
			id,
			source,
			settings: {
				output: outputSettings.get(),
				dither: ditherSettings.get(),
				colorSpace: colorSpace.get()
			},
			palette: selectedPalette.get(),
			settingsHash: hash
		});
	});
}

export async function processCurrentImage() {
	const hash = currentSettingsHash();
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
