import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { previewSettings, sourceImageData } from '$lib/stores/app';
import { addEffect, effectLayers, updateEffect } from '$lib/stores/effects';
import {
	adjustedSource,
	startSourceEffects,
	type SourceEffectsRequest,
	type SourceEffectsResponse
} from './source-effects';

class ControlledWorker {
	static instances: ControlledWorker[] = [];
	onmessage?: (event: MessageEvent<SourceEffectsResponse>) => void;
	messages: SourceEffectsRequest[] = [];
	terminate = vi.fn();
	constructor() {
		ControlledWorker.instances.push(this);
	}
	postMessage(message: SourceEffectsRequest) {
		this.messages.push(message);
	}
	reply(data: SourceEffectsResponse) {
		this.onmessage?.({ data } as MessageEvent<SourceEffectsResponse>);
	}
	get applied() {
		return this.messages.filter((message) => message.type === 'apply');
	}
}

const image = (): ImageData =>
	({ width: 1, height: 1, data: new Uint8ClampedArray(4) }) as ImageData;
const bitmap = () => ({ close: vi.fn() }) as unknown as ImageBitmap;

let stop: () => void;

beforeEach(() => {
	vi.stubGlobal('Worker', ControlledWorker);
	ControlledWorker.instances = [];
	effectLayers.set([]);
	sourceImageData.set(image());
	previewSettings.set({ sourceEffects: true });
	stop = startSourceEffects();
});

afterEach(() => {
	stop();
	vi.unstubAllGlobals();
});

describe('source effects', () => {
	it('runs one request at a time and folds edits during it into the next', () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		expect(worker.applied).toHaveLength(1);

		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 1 });
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 2 });
		expect(worker.applied).toHaveLength(1);

		worker.reply({ id: worker.applied[0]!.id, bitmap: bitmap() });
		expect(worker.applied).toHaveLength(2);
		expect(worker.applied[1]).toMatchObject({ effects: [{ stops: 2 }] });
	});

	it('drops a result computed for a source that was since replaced', () => {
		addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		const stale = bitmap();
		sourceImageData.set(image());
		worker.reply({ id: worker.applied[0]!.id, bitmap: stale });
		expect(adjustedSource.get()).toBeUndefined();
		expect(stale.close).toHaveBeenCalled();
	});

	it('stops the worker when turned off', () => {
		addEffect('exposure');
		previewSettings.set({ sourceEffects: false });
		expect(ControlledWorker.instances[0]!.terminate).toHaveBeenCalled();
	});
});
