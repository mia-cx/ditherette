import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { sourceImageData } from '$lib/stores/app';
import { addEffect, effectLayers, updateEffect } from '$lib/stores/effects';
import {
	compiledEffects,
	compiledEffectsFor,
	currentEffectsKey,
	startLiveEffects,
	type LiveEffectsRequest,
	type LiveEffectsResponse
} from './live-effects';

class ControlledWorker {
	static instances: ControlledWorker[] = [];
	onmessage?: (event: MessageEvent<LiveEffectsResponse>) => void;
	messages: LiveEffectsRequest[] = [];
	terminate = vi.fn();
	constructor() {
		ControlledWorker.instances.push(this);
	}
	postMessage(message: LiveEffectsRequest) {
		this.messages.push(message);
	}
	get compiles() {
		return this.messages.filter((message) => message.type === 'compile');
	}
	/** Answer the latest compile. */
	reply(results = new Uint32Array([1])) {
		const request = this.compiles.at(-1)!;
		this.onmessage?.({
			data: {
				type: 'compiled',
				id: request.id,
				sourceId: request.sourceId,
				key: request.key,
				results
			}
		} as MessageEvent<LiveEffectsResponse>);
	}
}

const image = () => ({ width: 1, height: 1, data: new Uint8ClampedArray(4) }) as ImageData;
let stop: () => void;

beforeEach(() => {
	vi.stubGlobal('Worker', ControlledWorker);
	ControlledWorker.instances = [];
	effectLayers.set([]);
	sourceImageData.set(image());
	stop = startLiveEffects();
});

afterEach(() => {
	stop();
	vi.unstubAllGlobals();
});

describe('live effects', () => {
	it('compiles as soon as effects change, and folds edits during a compile into the next', () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		expect(worker.messages[0]!.type).toBe('source');
		expect(worker.compiles).toHaveLength(1);

		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 1 });
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 2 });
		expect(worker.compiles).toHaveLength(1);

		worker.reply();
		expect(worker.compiles).toHaveLength(2);
		expect(worker.compiles[1]).toMatchObject({ effects: [{ stops: 2 }] });
	});

	it('hands processing the results for its key, and releases keys that moved on', async () => {
		const layer = addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		const source = sourceImageData.get()!;
		const first = compiledEffectsFor(source, currentEffectsKey());
		// Folded away while the first compile runs, so it never compiles.
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 1 });
		const skipped = compiledEffectsFor(source, currentEffectsKey());
		updateEffect(layer.id, { effect: 'exposure', enabled: true, stops: 2 });
		const current = compiledEffectsFor(source, currentEffectsKey());

		worker.reply(new Uint32Array([7]));
		await expect(first).resolves.toEqual(new Uint32Array([7]));
		await expect(skipped).rejects.toThrow('The effects changed.');
		worker.reply(new Uint32Array([9]));
		await expect(current).resolves.toEqual(new Uint32Array([9]));
		expect(compiledEffects.get()?.results).toEqual(new Uint32Array([9]));
		// Already compiled: answered at once.
		await expect(compiledEffectsFor(source, currentEffectsKey())).resolves.toEqual(
			new Uint32Array([9])
		);
	});

	it('loads a new source before compiling for it', () => {
		addEffect('exposure');
		const worker = ControlledWorker.instances[0]!;
		worker.reply();
		sourceImageData.set(image());
		expect(worker.messages.filter((message) => message.type === 'source')).toHaveLength(2);
		expect(worker.compiles).toHaveLength(2);
		expect(compiledEffects.get()).toBeUndefined();
	});
});
