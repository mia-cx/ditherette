import { resizeImageData } from '$lib/processing/resize';
import { quantizeImage } from '$lib/processing/quantize';
import { validateWorkerRequest } from '$lib/processing/schemas';
import { clampOutputSize, type WorkerResponse } from '$lib/processing/types';

const PROGRESS = {
	queued: 0.05,
	resizing: 0.18,
	quantizing: 0.62,
	finalizing: 0.95
} as const;

function progress(id: number, stage: string, value: number) {
	postMessage({ id, type: 'progress', stage, progress: value } satisfies WorkerResponse);
}

function requestIdFromMessage(value: unknown) {
	if (!value || typeof value !== 'object') return -1;
	const id = (value as { id?: unknown }).id;
	return Number.isInteger(id) ? (id as number) : -1;
}

self.onmessage = (event: MessageEvent<unknown>) => {
	let request;
	try {
		request = validateWorkerRequest(event.data);
	} catch (error) {
		postMessage({
			id: requestIdFromMessage(event.data),
			type: 'error',
			message:
				error instanceof Error ? error.message : 'Worker received an invalid processing request.'
		} satisfies WorkerResponse);
		return;
	}

	const { id, source, settings, palette, settingsHash } = request;
	try {
		progress(id, 'Sizing output', PROGRESS.queued);
		const size = clampOutputSize(settings.output.width, settings.output.height);
		progress(id, 'Resizing', PROGRESS.resizing);
		const resized = resizeImageData(
			source,
			size.width,
			size.height,
			settings.output.resize,
			settings.output.crop
		);
		progress(id, 'Quantizing palette', PROGRESS.quantizing);
		const result = quantizeImage(resized, palette, settings);
		const warnings = size.warning ? [size.warning, ...result.warnings] : result.warnings;
		progress(id, 'Finalizing indexed output', PROGRESS.finalizing);
		postMessage({
			id,
			type: 'complete',
			image: {
				...result,
				width: size.width,
				height: size.height,
				warnings,
				settingsHash,
				updatedAt: Date.now()
			}
		} satisfies WorkerResponse);
	} catch (error) {
		postMessage({
			id,
			type: 'error',
			message: error instanceof Error ? error.message : 'Processing failed'
		} satisfies WorkerResponse);
	}
};

export {};
