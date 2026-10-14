import { encodeIndexedPng } from '$lib/processing/png';
import type { PngWorkerRequest, PngWorkerResponse } from '$lib/processing/png-worker';

/** One export per worker; the client terminates it on completion or cancellation. */
self.onmessage = ({ data }: MessageEvent<PngWorkerRequest>) => {
	let response: PngWorkerResponse;
	try {
		response = { blob: encodeIndexedPng(data.image) };
	} catch (error) {
		response = { error: error instanceof Error ? error.message : 'PNG encoding failed.' };
	}
	self.postMessage(response);
};
