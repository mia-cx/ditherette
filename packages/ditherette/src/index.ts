import { DitheretteError } from './errors.js';
import { createCpu } from './cpu.js';
import { field, object, validateOptions } from './validation.js';
import type { Ditherette, InitOptions, CpuInitOptions, GpuInitOptions } from './types.js';
import type { GpuDither, GpuDitherOptions } from './gpu-dither.js';
import { fallbackPolicy } from './gpu-effects.js';

export { DitheretteError } from './errors.js';
export { createGpuEffects } from './gpu-effects.js';
export { createGpuDither } from './gpu-dither.js';
export type { DitherExecution, GpuDither, GpuDitherOptions } from './gpu-dither.js';
export type { EffectsExecution, GpuEffects, GpuEffectsOptions } from './gpu-effects.js';
export {
	applyCompiledEffects,
	compileEffectMask,
	compileEffects,
	indexColours
} from './compiled-effects.js';
export type {
	CompileEffectMaskRequest,
	CompileEffectsRequest,
	IndexedColours
} from './compiled-effects.js';
export { isEffect } from './validation-effects.js';
export type { ErrorCode } from './errors.js';
export type { ExecutionTiming } from './timings.js';
export type {
	AlphaPolicy,
	AnalyzePaletteFitRequest,
	AnalyzeRecolourRequest,
	ApplyEffectsRequest,
	BrightnessContrastEffect,
	ColourChannel,
	ColourModel,
	Curve,
	CurveGrid,
	CurvePoints,
	CurvesEffect,
	Effect,
	EffectChannel,
	EffectContext,
	EffectMaskRequest,
	FitLook,
	FitMeasurements,
	ExposureEffect,
	HueSaturationEffect,
	LevelsEffect,
	LevelsPoints,
	MaskCurve,
	OneInputCurve,
	OneInputMaskCurve,
	RecolourEffect,
	RecolourGroup,
	RecolourRecipe,
	WhiteBalanceEffect,
	RecipeV2,
	IndexedImage,
	Matching,
	PaletteEntry,
	PaletteFitEffect,
	PaletteFitSpace,
	QuantizeRequest,
	WorkingSpace,
	Field,
	Placement,
	PerturbPolicy,
	PerturbRequest,
	DitherAndQuantizeRequest,
	DiffusionKernel,
	RecipeV1,
	ProcessRequest,
	Ditherette,
	InitInput,
	InitOptions,
	CpuInitOptions,
	GpuInitOptions,
	TilingPolicy,
	Progress,
	ResizeAnchor,
	ResizeRequest,
	Rgba8Image,
	StepEnvelope,
	TwoInputCurve,
	TwoInputMaskCurve
} from './types.js';

/** Select one isolated CPU or GPU processor. Importing the package loads no Wasm or workers. */
export function createDitherette(options: GpuInitOptions): Promise<GpuDither>;
export function createDitherette(options?: CpuInitOptions): Promise<Ditherette>;
export function createDitherette(options?: InitOptions): Promise<Ditherette | GpuDither>;
export async function createDitherette(options?: InitOptions): Promise<Ditherette | GpuDither> {
	try {
		const input = object(
			options === undefined ? {} : options,
			[
				'engine',
				'threads',
				'tiling',
				'memoryLimitBytes',
				'wasm',
				'gpuWasm',
				'fallback',
				'diffusionBackend'
			],
			'invalid-settings',
			''
		);
		const selected = field(input, 'engine');
		const engine = selected === undefined ? 'cpu' : selected;
		if (engine !== 'cpu' && engine !== 'gpu')
			throw new DitheretteError('invalid-settings', 'engine', 'Expected cpu or gpu.');
		const normalized = validateOptions({
			threads: field(input, 'threads'),
			tiling: field(input, 'tiling'),
			memoryLimitBytes: field(input, 'memoryLimitBytes'),
			wasm: field(input, 'wasm')
		});
		if (engine === 'cpu') {
			object(
				input,
				['engine', 'threads', 'tiling', 'memoryLimitBytes', 'wasm'],
				'invalid-settings',
				''
			);
			return createCpu(normalized);
		}
		const selectedDiffusion = field(input, 'diffusionBackend');
		const diffusionBackend = selectedDiffusion === undefined ? 'gpu' : selectedDiffusion;
		if (diffusionBackend !== 'cpu' && diffusionBackend !== 'gpu')
			throw new DitheretteError('invalid-settings', 'diffusionBackend', 'Expected cpu or gpu.');
		const gpuOptions: GpuDitherOptions = {
			...normalized,
			gpuWasm: validateOptions({ wasm: field(input, 'gpuWasm') }).wasm,
			fallback: fallbackPolicy(field(input, 'fallback')),
			diffusionBackend
		};
		const { createGpuDither } = await import('./gpu-dither.js');
		return createGpuDither(gpuOptions);
	} catch (error) {
		if (error instanceof DitheretteError) throw error;
		throw new DitheretteError('invalid-settings', '', 'Initialization options could not be read.');
	}
}
