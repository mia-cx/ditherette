import type { ProcessedImage } from './types';

export type PngWorkerRequest = { image: ProcessedImage };
export type PngWorkerResponse = { blob: Blob } | { error: string };

/** Encode in a disposable worker, transferring a copy so displayed indices stay attached. */
export function encodeIndexedPngOffThread(
	image: ProcessedImage,
	signal?: AbortSignal
): Promise<Blob> {
	return new Promise((resolve, reject) => {
		if (signal?.aborted) return reject(signal.reason);
		let worker: Worker | undefined;
		function finish(blob?: Blob, error?: unknown) {
			signal?.removeEventListener('abort', abort);
			if (worker) {
				worker.onmessage = worker.onerror = worker.onmessageerror = null;
				worker.terminate();
				worker = undefined;
			}
			if (blob) resolve(blob);
			else reject(error);
		}
		function abort() {
			finish(undefined, signal?.reason);
		}
		try {
			worker = new Worker(new URL('../workers/png.worker.ts', import.meta.url), { type: 'module' });
			signal?.addEventListener('abort', abort, { once: true });
			worker.onmessage = ({ data }: MessageEvent<PngWorkerResponse>) => {
				if (!data || typeof data !== 'object')
					return finish(undefined, new Error('Invalid PNG worker response.'));
				if ('blob' in data && data.blob instanceof Blob && data.blob.type === 'image/png')
					finish(data.blob);
				else
					finish(
						undefined,
						new Error('error' in data ? data.error : 'Invalid PNG worker response.')
					);
			};
			worker.onerror = () => finish(undefined, new Error('The PNG worker failed.'));
			worker.onmessageerror = () =>
				finish(undefined, new Error('The PNG worker response could not be read.'));
			// Copy only the view, not a potentially larger backing buffer. Never transfer the live result.
			const indices = image.indices.slice();
			worker.postMessage({ image: { ...image, indices } } satisfies PngWorkerRequest, [
				indices.buffer
			]);
		} catch (error) {
			finish(undefined, error);
		}
	});
}
