import type { Ditherette } from 'ditherette';
import { initializePackageProcessor } from '$lib/processing/worker-pipeline';
import type { SourceEffectsRequest, SourceEffectsResponse } from '$lib/processing/source-effects';

/** Applies effects to the full source for the Source pane, apart from the processing worker. */
let source: ImageData | undefined;
let processor: Promise<Ditherette> | undefined;

self.onmessage = async ({ data }: MessageEvent<SourceEffectsRequest>) => {
	if (data.type === 'source') {
		source = data.source;
		return;
	}
	const reply = (response: SourceEffectsResponse, transfer: Transferable[] = []) =>
		self.postMessage(response, { transfer });
	try {
		processor ??= initializePackageProcessor().catch((error: unknown) => {
			processor = undefined;
			throw error;
		});
		const ditherette = await processor;
		if (!source) throw new Error('Source is not loaded.');
		const image = ditherette.applyEffects({
			version: 1,
			source: {
				width: source.width,
				height: source.height,
				data: new Uint8Array(source.data.buffer, source.data.byteOffset, source.data.byteLength)
			},
			effects: data.effects,
			context: data.context
		});
		// applyEffects returns independent JS-owned bytes, never shared Wasm memory.
		const buffer = image.data.buffer as ArrayBuffer;
		const pixels = new Uint8ClampedArray(buffer, image.data.byteOffset, image.data.byteLength);
		const bitmap = await createImageBitmap(new ImageData(pixels, image.width, image.height));
		reply({ id: data.id, bitmap }, [bitmap]);
	} catch (error) {
		reply({ id: data.id, error: error instanceof Error ? error.message : 'Effects failed.' });
	}
};
