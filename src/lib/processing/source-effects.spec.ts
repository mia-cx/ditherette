import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { previewSettings, sourceImageData } from '$lib/stores/app';
import { addEffect, effectLayers, updateEffect } from '$lib/stores/effects';
import {
	sourceEffectsLut,
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
const lut = () => new Uint8Array(4);

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

		worker.reply({ id: worker.applied[0]!.id, lut: lut() });
		expect(worker.applied).toHaveLength(2);
		expect(worker.applied[1]).toMatchObject({ effects: [{ stops: 2 }] });
	});

	it('drops a result computed for a source that was since replaced', () => {
		addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		sourceImageData.set(image());
		worker.reply({ id: worker.applied[0]!.id, lut: lut() });
		expect(sourceEffectsLut.get()).toBeUndefined();
		// The stale reply frees the worker for the new source's table.
		expect(worker.applied).toHaveLength(2);
	});

	it('keeps the table while turned off, and shows it again without recomputing', () => {
		addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		const table = lut();
		worker.reply({ id: worker.applied[0]!.id, lut: table });
		previewSettings.set({ sourceEffects: false });
		previewSettings.set({ sourceEffects: true });
		expect(worker.terminate).not.toHaveBeenCalled();
		expect(worker.applied).toHaveLength(1);
		expect(sourceEffectsLut.get()).toBe(table);
	});

	it('reuses the table when an effect is hidden and shown again', () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		const table = lut();
		worker.reply({ id: worker.applied[0]!.id, lut: table });
		updateEffect(layer.id, { ...layer.step, enabled: false });
		expect(sourceEffectsLut.get()).toBeUndefined();
		updateEffect(layer.id, { ...layer.step, enabled: true });
		expect(worker.applied).toHaveLength(1);
		expect(sourceEffectsLut.get()).toBe(table);
	});
});
