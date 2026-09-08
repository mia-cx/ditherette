import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	cancelProcessing,
	currentSettingsHash,
	processCurrentImage,
	scheduleProcessing
} from './client';
import {
	outputSettings,
	processedImage,
	processingError,
	processingProgress,
	sourceImageData
} from '$lib/stores/app';
import type { ProcessedImage, WorkerRequest } from './types';

// The browser transport is the boundary. Stores, scheduling, validation, and persistence calls stay real.
class ControlledWorker {
	static instances: ControlledWorker[] = [];
	onmessage?: (event: MessageEvent<unknown>) => void;
	onerror?: () => void;
	messages: WorkerRequest[] = [];
	terminate = vi.fn();
	constructor() {
		ControlledWorker.instances.push(this);
	}
	postMessage(message: WorkerRequest) {
		this.messages.push(message);
	}
	receive(data: unknown) {
		this.onmessage?.({ data } as MessageEvent<unknown>);
	}
}

function source(): ImageData {
	return { width: 1, height: 1, data: new Uint8ClampedArray([0, 0, 0, 255]), colorSpace: 'srgb' };
}

function preview(settingsHash = 'previous'): ProcessedImage {
	return {
		width: 1,
		height: 1,
		indices: new Uint8Array([0]),
		palette: [],
		transparentIndex: -1,
		warnings: [],
		settingsHash,
		updatedAt: 1
	};
}

beforeEach(() => {
	vi.useFakeTimers();
	vi.stubGlobal('Worker', ControlledWorker);
	ControlledWorker.instances = [];
	sourceImageData.set(source());
	processedImage.set(preview());
	processingError.set(undefined);
	processingProgress.set(undefined);
	outputSettings.set({ ...outputSettings.get(), width: 1, height: 1 });
});

afterEach(() => {
	cancelProcessing();
	vi.useRealTimers();
	vi.unstubAllGlobals();
});

describe('website processing scheduling', () => {
	it('invalidates stale events immediately and replaces active work after the slider debounce', async () => {
		const pending = processCurrentImage();
		await vi.advanceTimersByTimeAsync(0);
		const old = ControlledWorker.instances[0];
		const id = old.messages[0].id;
		const retained = processedImage.get();
		old.receive({
			id,
			type: 'progress',
			stage: 'quantize',
			progress: 0.2,
			completed: 2,
			total: 10
		});
		expect(processingProgress.get()).toEqual({
			stage: 'quantize',
			progress: 0.2,
			completed: 2,
			total: 10
		});
		outputSettings.set({ ...outputSettings.get(), width: 2 });
		scheduleProcessing(180);
		old.receive({ id, type: 'progress', stage: 'Stale progress', progress: 0.9 });
		expect(processingProgress.get()).toBeUndefined();
		expect(processedImage.get()).toBe(retained);
		expect(old.terminate).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(179);
		expect(ControlledWorker.instances).toHaveLength(1);
		await vi.advanceTimersByTimeAsync(1);
		expect(old.terminate).toHaveBeenCalledOnce();
		expect(ControlledWorker.instances).toHaveLength(2);
		expect(old.messages.some(({ type }) => type === 'cancel')).toBe(false);
		const activeProgress = processingProgress.get();
		old.receive({ malformed: true });
		old.onerror?.();
		expect(processingProgress.get()).toBe(activeProgress);
		expect(processingError.get()).toBeUndefined();
		await pending;
	});

	it('ignores a stale malformed response ID before validating it on a reused worker', async () => {
		const first = processCurrentImage();
		await vi.advanceTimersByTimeAsync(0);
		const worker = ControlledWorker.instances[0];
		const oldId = worker.messages[0].id;
		worker.receive({ id: oldId, type: 'error', message: 'Visible processing failure' });
		await first;
		expect(processingError.get()).toBe('Visible processing failure');
		const second = processCurrentImage();
		await vi.advanceTimersByTimeAsync(0);
		const activeProgress = processingProgress.get();
		worker.receive({ id: oldId, type: 'progress', progress: 'malformed' });
		expect(processingProgress.get()).toBe(activeProgress);
		expect(processingError.get()).toBeUndefined();
		cancelProcessing();
		await second;
	});

	it('explicit cancel terminates once and retains the last valid preview', async () => {
		const pending = processCurrentImage();
		await vi.advanceTimersByTimeAsync(0);
		const worker = ControlledWorker.instances[0];
		const retained = processedImage.get();
		cancelProcessing();
		cancelProcessing();
		worker.receive({ id: worker.messages[0].id, type: 'error', message: 'Late error' });
		worker.onerror?.();
		await pending;
		expect(worker.terminate).toHaveBeenCalledOnce();
		expect(processedImage.get()).toBe(retained);
		expect(processingProgress.get()).toBeUndefined();
		expect(processingError.get()).toBeUndefined();
	});

	it('drops a pending replacement when settings return to the retained preview', async () => {
		processedImage.set(preview(currentSettingsHash()));
		const original = outputSettings.get();
		outputSettings.set({ ...original, width: 2 });
		scheduleProcessing(180);
		outputSettings.set(original);
		scheduleProcessing(180);
		await vi.advanceTimersByTimeAsync(180);
		expect(ControlledWorker.instances).toHaveLength(0);
		expect(processingError.get()).toBeUndefined();
	});
});
