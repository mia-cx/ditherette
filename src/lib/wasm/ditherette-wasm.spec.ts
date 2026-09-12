import { describe, expect, it } from 'vitest';
import {
	canResizeWholeImageWithWasm,
	wasmResizeRequest,
	wasmThreadsAvailable
} from './ditherette-wasm';

class TestImageData implements ImageData {
	readonly data: Uint8ClampedArray<ArrayBuffer>;
	readonly colorSpace: PredefinedColorSpace = 'srgb';

	constructor(
		data: Uint8ClampedArray<ArrayBuffer>,
		readonly width: number,
		readonly height: number
	) {
		this.data = data;
	}
}

Object.defineProperty(globalThis, 'ImageData', { value: TestImageData, configurable: true });

describe('ditherette wasm wrapper', () => {
	it('maps resize modes to the generic Wasm export arguments', () => {
		expect(wasmResizeRequest('nearest')).toEqual({ filter: 'nearest', supportPolicy: 'fixed' });
		expect(wasmResizeRequest('area')).toEqual({ filter: 'area', supportPolicy: 'fixed' });
		expect(wasmResizeRequest('lanczos2-scale-aware')).toEqual({
			filter: 'lanczos2',
			supportPolicy: 'scale-aware'
		});
		expect(wasmResizeRequest('lanczos3-scale-aware')).toEqual({
			filter: 'lanczos3',
			supportPolicy: 'scale-aware'
		});
	});

	it('reports thread availability from browser shared-memory capabilities', () => {
		const originalCrossOriginIsolated = Object.getOwnPropertyDescriptor(
			globalThis,
			'crossOriginIsolated'
		);
		Object.defineProperty(globalThis, 'crossOriginIsolated', { value: false, configurable: true });
		expect(wasmThreadsAvailable()).toBe(false);
		Object.defineProperty(globalThis, 'crossOriginIsolated', { value: true, configurable: true });
		expect(wasmThreadsAvailable()).toBe(typeof SharedArrayBuffer !== 'undefined');
		if (originalCrossOriginIsolated) {
			Object.defineProperty(globalThis, 'crossOriginIsolated', originalCrossOriginIsolated);
		}
	});

	it('only sends whole-image resizes to the current Wasm resize export', () => {
		const source = new ImageData(new Uint8ClampedArray(10 * 20 * 4), 10, 20);

		expect(canResizeWholeImageWithWasm(source)).toBe(true);
		expect(canResizeWholeImageWithWasm(source, { x: 0, y: 0, width: 10, height: 20 })).toBe(true);
		expect(canResizeWholeImageWithWasm(source, { x: 1, y: 0, width: 9, height: 20 })).toBe(false);
		expect(canResizeWholeImageWithWasm(source, { x: 0, y: 0, width: 9, height: 20 })).toBe(false);
	});
});
