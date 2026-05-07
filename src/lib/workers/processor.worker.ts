import { resizeImageData } from '$lib/processing/resize';
import { quantizeImage } from '$lib/processing/quantize';
import { clampOutputSize, type WorkerRequest, type WorkerResponse } from '$lib/processing/types';

function progress(id: number, stage: string, value: number) {
	postMessage({ id, type: 'progress', stage, progress: value } satisfies WorkerResponse);
}

function isWorkerRequest(value: unknown): value is WorkerRequest {
	if (!value || typeof value !== 'object') return false;
	const request = value as Partial<WorkerRequest>;
	return (
		typeof request.id === 'number' &&
		request.source instanceof ImageData &&
		Boolean(request.settings?.output) &&
		Boolean(request.settings?.dither) &&
		typeof request.settingsHash === 'string' &&
		Array.isArray(request.palette)
	);
}

self.onmessage = (event: MessageEvent<unknown>) => {
	if (event.origin && event.origin !== self.location.origin) return;
	if (!isWorkerRequest(event.data)) {
		postMessage({
			id: -1,
			type: 'error',
			message: 'Worker received an invalid processing request.'
		} satisfies WorkerResponse);
		return;
	}

	const { id, source, settings, palette, settingsHash } = event.data;
	try {
		progress(id, 'Sizing output', 0.05);
		const size = clampOutputSize(settings.output.width, settings.output.height);
		progress(id, 'Resizing', 0.18);
		const resized = resizeImageData(
			source,
			size.width,
			size.height,
			settings.output.fit,
			settings.output.resize,
			settings.output.crop
		);
		progress(id, 'Quantizing palette', 0.62);
		const result = quantizeImage(resized, palette, settings);
		const warnings = size.warning ? [size.warning, ...result.warnings] : result.warnings;
		progress(id, 'Finalizing indexed output', 0.95);
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
