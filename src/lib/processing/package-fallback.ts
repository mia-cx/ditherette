import { packageProcessRequest } from './package-adapter';
import { clampCrop } from './resize';
import { clampOutputSize, type EnabledPaletteColor, type ProcessingSettings } from './types';

export const FALLBACK_WARNING =
	'Wasm initialization failed. Using faithful TypeScript fallback for this page session.';

/** A package load or supported initialization failure, distinct from a processing failure. */
export class PackageInitializationFailure extends Error {
	constructor(cause: unknown) {
		super('Wasm could not initialize.', { cause });
		this.name = 'PackageInitializationFailure';
	}
}

/** Loading may fall back. Typed processing, validation, and memory failures never do. */
export async function initializePackageProcessor() {
	let module;
	try {
		module = await import('ditherette');
	} catch (error) {
		throw new PackageInitializationFailure(error);
	}
	try {
		return await module.createDitherette();
	} catch (error) {
		if (
			error instanceof module.DitheretteError &&
			(error.code === 'initialization' || error.code === 'capability')
		)
			throw new PackageInitializationFailure(error);
		throw error;
	}
}

/** Admit only requests whose index selection needs no unproven floating-point equivalence. */
export function faithfulTypeScriptFallback(
	source: Pick<ImageData, 'width' | 'height' | 'data'>,
	palette: EnabledPaletteColor[],
	settings: ProcessingSettings
): boolean {
	if (
		settings.output.resize !== 'nearest' ||
		settings.output.alphaMode !== 'preserve' ||
		settings.colorSpace !== 'srgb' ||
		settings.dither.algorithm !== 'none'
	)
		return false;
	const size = clampOutputSize(settings.output.width, settings.output.height);
	const { request } = packageProcessRequest(source, palette, settings, size);
	const crop = clampCrop(source.width, source.height, settings.output.crop);
	for (const [length, output, origin] of [
		[crop.width, size.width, crop.x],
		[crop.height, size.height, crop.y]
	]) {
		for (let coordinate = 0; coordinate < output; coordinate++) {
			// Compare the current website floating center map with the package's integer center map.
			const website = Math.round(origin + (coordinate + 0.5) * (length / output) - 0.5);
			const canonical = origin + Math.floor(((coordinate * 2 + 1) * length) / (output * 2));
			if (website !== canonical) return false;
		}
	}
	const visible = new Set(
		request.palette
			.slice(0, 256)
			.flatMap((entry) => (entry.kind === 'color' ? [entry.rgb.join(',')] : []))
	);
	if (visible.size <= 1) return true;
	const bytes = request.source.data;
	for (let offset = 0; offset < bytes.length; offset += 4) {
		if (bytes[offset + 3] <= settings.output.alphaThreshold) continue;
		if (!visible.has(`${bytes[offset]},${bytes[offset + 1]},${bytes[offset + 2]}`)) return false;
	}
	return true;
}
