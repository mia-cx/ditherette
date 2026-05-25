import { resizeImageData } from '$lib/processing/resize';
import type { CropRect, ProcessingSettings, ResizeId } from '$lib/processing/types';

const WASM_MODULE_URL = '/wasm/ditherette-wasm/ditherette_wasm.js';
const SOURCE_COLOR_SPACE = 'rgba8';
const CENTER_ANCHOR = 'center';
const FIXED_SUPPORT = 'fixed';
const SCALE_AWARE_SUPPORT = 'scale-aware';

export type WasmColorSpaceF32 =
	| 'srgb-f32'
	| 'linear-srgb-f32'
	| 'oklab-f32'
	| 'oklch-f32'
	| 'cielab-f32'
	| 'cielch-f32'
	| 'ycbcr-f32';

export type ResizeEngine = 'typescript' | 'wasm scalar';

type DitheretteWasmModule = {
	default: (moduleOrPath?: unknown) => Promise<unknown>;
	convertColorSpace: (
		input: Uint8Array,
		width: number,
		height: number,
		from: string,
		to: string,
		parallelizationPolicy: boolean
	) => Float32Array;
	resizeRgba8: (
		input: Uint8Array,
		sourceWidth: number,
		sourceHeight: number,
		outputWidth: number,
		outputHeight: number,
		filter: string,
		anchor: string,
		supportPolicy: string,
		parallelizationPolicy: boolean
	) => Uint8Array;
	processRgba8: (
		input: Uint8Array,
		width: number,
		height: number,
		settingsJson: string,
		parallelizationPolicy: boolean
	) => Uint8Array;
};

type ResizeRequest = {
	filter: string;
	supportPolicy: string;
};

type ResizeResult = {
	image: ImageData;
	engine: ResizeEngine;
};

let wasmModulePromise: Promise<DitheretteWasmModule | undefined> | undefined;
let warnedAboutLoadFailure = false;

export function wasmResizeEnabled() {
	return isTruthyFlag(import.meta.env.VITE_DITHERETTE_WASM_RESIZE);
}

export function wasmResizeRequest(mode: ResizeId): ResizeRequest {
	if (mode === 'lanczos2-scale-aware') {
		return { filter: 'lanczos2', supportPolicy: SCALE_AWARE_SUPPORT };
	}
	if (mode === 'lanczos3-scale-aware') {
		return { filter: 'lanczos3', supportPolicy: SCALE_AWARE_SUPPORT };
	}
	return { filter: mode, supportPolicy: FIXED_SUPPORT };
}

export function canResizeWholeImageWithWasm(source: ImageData, crop?: CropRect) {
	return (
		!crop ||
		(crop.x === 0 && crop.y === 0 && crop.width === source.width && crop.height === source.height)
	);
}

export async function convertColorSpaceF32(
	source: ImageData,
	target: WasmColorSpaceF32,
	parallelizationPolicy = true
): Promise<Float32Array> {
	const wasm = await loadDitheretteWasm();
	if (!wasm) throw new Error('Ditherette Wasm module is not available. Run `pnpm wasm:build`.');
	return wasm.convertColorSpace(
		imageDataBytes(source),
		source.width,
		source.height,
		SOURCE_COLOR_SPACE,
		target,
		parallelizationPolicy
	);
}

export async function processImageDataWithWasm(
	source: ImageData,
	settings: ProcessingSettings,
	parallelizationPolicy = true
): Promise<ImageData> {
	const wasm = await loadDitheretteWasm();
	if (!wasm) throw new Error('Ditherette Wasm module is not available. Run `pnpm wasm:build`.');
	const output = wasm.processRgba8(
		imageDataBytes(source),
		source.width,
		source.height,
		JSON.stringify(settings),
		parallelizationPolicy
	);
	return new ImageData(
		new Uint8ClampedArray(output),
		settings.output.width,
		settings.output.height
	);
}

export async function resizeImageDataWithOptionalWasm(
	source: ImageData,
	width: number,
	height: number,
	mode: ResizeId,
	crop?: CropRect
): Promise<ResizeResult> {
	const wasmImage = await tryResizeImageDataWithWasm(source, width, height, mode, crop);
	if (wasmImage) return { image: wasmImage, engine: 'wasm scalar' };
	return { image: resizeImageData(source, width, height, mode, crop), engine: 'typescript' };
}

export async function tryResizeImageDataWithWasm(
	source: ImageData,
	width: number,
	height: number,
	mode: ResizeId,
	crop?: CropRect
): Promise<ImageData | undefined> {
	if (!wasmResizeEnabled() || !canResizeWholeImageWithWasm(source, crop)) return undefined;

	const wasm = await loadDitheretteWasm();
	if (!wasm) return undefined;

	const { filter, supportPolicy } = wasmResizeRequest(mode);
	const output = wasm.resizeRgba8(
		imageDataBytes(source),
		source.width,
		source.height,
		width,
		height,
		filter,
		CENTER_ANCHOR,
		supportPolicy,
		true
	);

	return new ImageData(new Uint8ClampedArray(output), width, height);
}

function imageDataBytes(source: ImageData) {
	return new Uint8Array(source.data.buffer, source.data.byteOffset, source.data.byteLength);
}

function isTruthyFlag(value: unknown) {
	return value === true || value === 'true' || value === '1' || value === 'yes';
}

async function loadDitheretteWasm() {
	wasmModulePromise ??= import(/* @vite-ignore */ WASM_MODULE_URL)
		.then(async (wasm: DitheretteWasmModule) => {
			await wasm.default();
			return wasm;
		})
		.catch((error: unknown) => {
			if (!warnedAboutLoadFailure) {
				warnedAboutLoadFailure = true;
				console.warn('Ditherette Wasm module failed to load; falling back to TypeScript.', error);
			}
			return undefined;
		});
	return wasmModulePromise;
}
