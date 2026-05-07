import { Effect } from 'effect';
import {
	colorSpace,
	ditherSettings,
	outputSettings,
	paletteEnabled,
	processedImage,
	processingError,
	processingProgress,
	selectedPalette,
	sourceImageData
} from '$lib/stores/app';
import { saveProcessedImage, settingsHash } from './db';
import type { ProcessedImage, WorkerResponse } from './types';

let worker: Worker | undefined;
let activeReject: ((error: Error) => void) | undefined;
let requestId = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let stopAuto: (() => void) | undefined;

function getWorker() {
	activeReject?.(new Error('Processing was superseded by newer settings.'));
	activeReject = undefined;
	worker?.terminate();
	worker = new Worker(new URL('../workers/processor.worker.ts', import.meta.url), {
		type: 'module'
	});
	return worker;
}

function snapshotHash() {
	return settingsHash({
		output: outputSettings.get(),
		dither: ditherSettings.get(),
		colorSpace: colorSpace.get(),
		palette: paletteEnabled.get()
	});
}

function processInWorker(): Promise<ProcessedImage> {
	const source = sourceImageData.get();
	if (!source) return Promise.reject(new Error('Upload an image before processing.'));

	const id = ++requestId;
	const activeWorker = getWorker();
	const hash = snapshotHash();
	processingProgress.set({ stage: 'Queued', progress: 0 });
	processingError.set(undefined);

	return new Promise((resolve, reject) => {
		activeReject = reject;
		const settle = <T>(callback: (value: T) => void, value: T) => {
			if (activeReject === reject) activeReject = undefined;
			callback(value);
		};

		activeWorker.onmessage = (event: MessageEvent<WorkerResponse>) => {
			const message = event.data;
			if (message.id !== id) return;
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
	const hash = snapshotHash();
	const program = Effect.tryPromise({
		try: processInWorker,
		catch: (error) => (error instanceof Error ? error : new Error('Processing failed'))
	});

	try {
		const result = await Effect.runPromise(program);
		if (result.settingsHash !== hash || result.settingsHash !== snapshotHash()) return;
		processedImage.set(result);
		processingProgress.set(undefined);
		await saveProcessedImage(result);
	} catch (error) {
		const message = error instanceof Error ? error.message : 'Processing failed';
		processingError.set(message);
		processedImage.set(undefined);
	}
}

export function scheduleProcessing(delay = 180) {
	if (!sourceImageData.get()) return;
	if (timer) clearTimeout(timer);
	timer = setTimeout(() => {
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
		paletteEnabled.subscribe(() => scheduleProcessing())
	];
	stopAuto = () => {
		for (const unsubscribe of unsubscribers) unsubscribe();
		if (timer) clearTimeout(timer);
		activeReject?.(new Error('Processing was stopped.'));
		activeReject = undefined;
		worker?.terminate();
		worker = undefined;
		stopAuto = undefined;
	};
	return stopAuto;
}
