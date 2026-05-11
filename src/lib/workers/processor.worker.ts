import { validateWorkerRequest } from '$lib/processing/schemas';
import type { WorkerResponse } from '$lib/processing/types';
import {
	ProcessorWorkerPipeline,
	transferablesForWorkerResponse
} from '$lib/processing/worker-pipeline';

type WorkerPostTarget = {
	postMessage(message: unknown, transfer: Transferable[]): void;
	postMessage(message: unknown): void;
};

const pipeline = new ProcessorWorkerPipeline();
const workerSelf = self as unknown as WorkerPostTarget;
let workQueue = Promise.resolve();

function requestIdFromMessage(value: unknown) {
	if (!value || typeof value !== 'object') return -1;
	const id = (value as { id?: unknown }).id;
	return Number.isInteger(id) ? (id as number) : -1;
}

function postResponse(response: WorkerResponse | undefined) {
	if (!response) return;
	workerSelf.postMessage(response, transferablesForWorkerResponse(response));
}

self.onmessage = (event: MessageEvent<unknown>) => {
	let request;
	try {
		request = validateWorkerRequest(event.data);
	} catch (error) {
		workerSelf.postMessage({
			id: requestIdFromMessage(event.data),
			type: 'error',
			message:
				error instanceof Error ? error.message : 'Worker received an invalid processing request.'
		} satisfies WorkerResponse);
		return;
	}

	if (request.type === 'cancel') {
		pipeline.handle(request, () => undefined);
		return;
	}

	workQueue = workQueue.then(() => handleRequest(request));
};

async function handleRequest(
	request: Exclude<ReturnType<typeof validateWorkerRequest>, { type: 'cancel' }>
) {
	try {
		postResponse(
			await pipeline.handleAsync(request, (stage, progress) => {
				workerSelf.postMessage({
					id: request.id,
					type: 'progress',
					stage,
					progress
				} satisfies WorkerResponse);
			})
		);
	} catch (error) {
		workerSelf.postMessage({
			id: request.id,
			type: 'error',
			message: error instanceof Error ? error.message : 'Processing failed'
		} satisfies WorkerResponse);
	}
}

export {};
