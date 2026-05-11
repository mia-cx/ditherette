import { afterEach, describe, expect, it, vi } from 'vitest';
import { canUseRowWorkerQuantize, quantizeImageWithRowWorkers } from './quantize-row-workers';
import type {
	DitherSettings,
	EnabledPaletteColor,
	OutputSettings,
	ProcessingSettings
} from './types';

class TestImageData implements ImageData {
	readonly data: Uint8ClampedArray<ArrayBuffer>;
	readonly width: number;
	readonly height: number;
	readonly colorSpace: PredefinedColorSpace = 'srgb';

	constructor(data: Uint8ClampedArray<ArrayBuffer>, width: number, height: number) {
		this.data = data;
		this.width = width;
		this.height = height;
	}
}

type Listener = (event: MessageEvent<unknown>) => void;

class HangingWorker {
	static terminated = 0;
	readonly listeners = new Set<Listener>();

	constructor(_url: URL, _options: WorkerOptions) {}

	addEventListener(type: string, listener: EventListenerOrEventListenerObject) {
		if (type !== 'message') return;
		this.listeners.add(listener as Listener);
	}

	removeEventListener(type: string, listener: EventListenerOrEventListenerObject) {
		if (type !== 'message') return;
		this.listeners.delete(listener as Listener);
	}

	postMessage(_message: unknown) {}

	terminate() {
		HangingWorker.terminated++;
	}
}

const originalWorker = globalThis.Worker;

Object.defineProperty(globalThis, 'ImageData', { value: TestImageData, configurable: true });

const palette: EnabledPaletteColor[] = [
	{ name: 'Black', key: '#000000', rgb: { r: 0, g: 0, b: 0 }, kind: 'free', enabled: true },
	{ name: 'White', key: '#FFFFFF', rgb: { r: 255, g: 255, b: 255 }, kind: 'free', enabled: true }
];

const output: OutputSettings = {
	width: 4,
	height: 4,
	lockAspect: true,
	resize: 'nearest',
	alphaMode: 'preserve',
	alphaThreshold: 0,
	matteKey: '#FFFFFF',
	autoSizeOnUpload: false,
	scaleFactor: 1
};

const dither: DitherSettings = {
	algorithm: 'bayer-2',
	strength: 100,
	placement: 'everywhere',
	placementRadius: 3,
	placementThreshold: 12,
	placementSoftness: 8,
	serpentine: true,
	seed: 1,
	useColorSpace: false
};

const settings: ProcessingSettings = { output, dither, colorSpace: 'srgb' };

afterEach(() => {
	vi.restoreAllMocks();
	HangingWorker.terminated = 0;
	Object.defineProperty(globalThis, 'Worker', {
		value: originalWorker,
		configurable: true,
		writable: true
	});
});

describe('row-worker quantization', () => {
	it('only enables pixel-independent algorithms above the row-worker size gate', () => {
		Object.defineProperty(globalThis, 'Worker', {
			value: HangingWorker,
			configurable: true,
			writable: true
		});

		expect(canUseRowWorkerQuantize(settings, 16, 16)).toBe(true);
		expect(
			canUseRowWorkerQuantize({ ...settings, dither: { ...dither, algorithm: 'random' } }, 16, 16)
		).toBe(true);
		expect(
			canUseRowWorkerQuantize(
				{ ...settings, dither: { ...dither, algorithm: 'floyd-steinberg' } },
				16,
				16
			)
		).toBe(false);
		expect(canUseRowWorkerQuantize(settings, 15, 16)).toBe(false);
	});

	it('rejects cancellation while row workers are active instead of hanging', async () => {
		Object.defineProperty(globalThis, 'Worker', {
			value: HangingWorker,
			configurable: true,
			writable: true
		});
		let canceled = false;
		setTimeout(() => {
			canceled = true;
		}, 0);

		await expect(
			quantizeImageWithRowWorkers(
				new ImageData(new Uint8ClampedArray(4 * 4 * 4), 4, 4),
				palette,
				settings,
				undefined,
				{
					minPixels: 1,
					workerCount: 2,
					shouldCancel: () => canceled
				}
			)
		).rejects.toThrow(/canceled/i);
		expect(HangingWorker.terminated).toBeGreaterThan(0);
	});
});
