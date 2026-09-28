import type { Ditherette } from 'ditherette';
import {
	renderDitherPreview,
	type DitherPreviewRequest,
	type DitherPreviewResponse
} from '$lib/processing/dither-preview';
import { initializePackageProcessor } from '$lib/processing/worker-pipeline';

/** Renders dither previews one at a time, away from the page. */
let processor: Promise<Ditherette> | undefined;

self.onmessage = async ({ data }: MessageEvent<DitherPreviewRequest>) => {
	const reply = (response: DitherPreviewResponse, transfer: Transferable[] = []) =>
		self.postMessage(response, { transfer });
	try {
		processor ??= initializePackageProcessor().catch((error: unknown) => {
			processor = undefined;
			throw error;
		});
		const pixels = renderDitherPreview(await processor, data.params, data.size);
		reply({ id: data.id, pixels }, [pixels.buffer]);
	} catch (error) {
		reply({ id: data.id, error: error instanceof Error ? error.message : 'Preview failed.' });
	}
};
