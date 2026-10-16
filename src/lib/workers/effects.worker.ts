import { EffectsWorkerPipeline } from '$lib/processing/worker-pipeline';
import type { LiveEffectsRequest } from '$lib/processing/live-effects';

const pipeline = new EffectsWorkerPipeline((response, transfer = []) =>
	self.postMessage(response, { transfer })
);

self.onmessage = ({ data }: MessageEvent<LiveEffectsRequest>) => {
	void pipeline.handle(data);
};
