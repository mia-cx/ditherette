import { resizeImageData } from '../src/lib/processing/resize';
import { quantizeImage } from '../src/lib/processing/quantize';
import { faithfulTypeScriptFallback } from '../src/lib/processing/package-fallback';
import type { EnabledPaletteColor, ProcessingSettings } from '../src/lib/processing/types';
import type {
	IndexedImage,
	ProcessRequest,
	QuantizeRequest,
	ResizeAnchor,
	Rgba8Image
} from '../packages/ditherette/src/types';

/** Developer-only recipes registered by the paired browser protocol. */
interface BenchmarkResizeRequest {
	source: Rgba8Image;
	output: {
		width: number;
		height: number;
		resize:
			| { algorithm: 'area' }
			| { algorithm: 'nearest' | 'bilinear'; anchor: ResizeAnchor }
			| {
					algorithm: 'lanczos2' | 'lanczos3';
					anchor: ResizeAnchor;
					support: 'fixed' | 'scale-aware';
			  };
	};
}

/** Actual website resize, with durable output matching the package ownership boundary. */
export function resize(request: BenchmarkResizeRequest): Rgba8Image {
	if ('anchor' in request.output.resize && request.output.resize.anchor !== 'center') {
		throw new Error('The TypeScript implementation supports only center alignment.');
	}
	const { source, output } = request;
	if (!(source.data.buffer instanceof ArrayBuffer))
		throw new Error('Expected unshared RGBA8 input.');
	const input = new ImageData(
		new Uint8ClampedArray(source.data.buffer, source.data.byteOffset, source.data.byteLength),
		source.width,
		source.height
	);
	const recipe = output.resize;
	const mode =
		'support' in recipe && recipe.support === 'scale-aware'
			? (`${recipe.algorithm}-scale-aware` as const)
			: recipe.algorithm;
	const result = resizeImageData(input, output.width, output.height, mode);
	// The website intentionally aliases identity output. The public comparison must own its bytes.
	const data = result === input ? new Uint8Array(result.data) : new Uint8Array(result.data.buffer);
	return { width: result.width, height: result.height, data };
}

function websiteRequest(request: QuantizeRequest | ProcessRequest) {
	const recipe = 'recipe' in request ? request.recipe : undefined;
	const alpha = recipe?.alpha ?? ('alpha' in request ? request.alpha : undefined);
	const matching = recipe?.match ?? ('matching' in request ? request.matching : undefined);
	if (
		alpha?.mode !== 'preserve' ||
		matching !== 'srgb-euclidean' ||
		(recipe &&
			(recipe.dither.family !== 'none' ||
				recipe.output.resize.algorithm !== 'nearest' ||
				recipe.output.resize.anchor !== 'center'))
	)
		throw new Error(
			'TypeScript indexed comparison requires nearest/no-dither/sRGB/preserve alpha.'
		);
	// Restrict this adapter to warning-free normalization. Other metadata needs its own proof.
	if (
		request.palette.length > 256 ||
		!request.palette.some((entry) => entry.kind === 'color') ||
		request.palette.filter((entry) => entry.kind === 'transparent').length !== 1
	)
		throw new Error(
			'TypeScript indexed comparison requires visible colors and one explicit transparent entry, at most 256 entries.'
		);
	const palette: EnabledPaletteColor[] = request.palette.map((entry, index) => ({
		name: `entry-${index}`,
		key: `entry-${index}`,
		enabled: true,
		kind: entry.kind === 'transparent' ? 'transparent' : 'custom',
		rgb: entry.kind === 'color' ? { r: entry.rgb[0], g: entry.rgb[1], b: entry.rgb[2] } : undefined
	}));
	const settings: ProcessingSettings = {
		output: {
			width: recipe?.output.width ?? request.source.width,
			height: recipe?.output.height ?? request.source.height,
			resize: 'nearest',
			alphaMode: 'preserve',
			alphaThreshold: alpha.threshold,
			matteKey: palette.find((entry) => entry.rgb)!.key,
			lockAspect: false,
			autoSizeOnUpload: false,
			scaleFactor: 1
		},
		colorSpace: 'srgb',
		dither: {
			algorithm: 'none',
			strength: 100,
			placement: 'everywhere',
			placementRadius: 1,
			placementThreshold: 0,
			placementSoftness: 0,
			serpentine: false,
			seed: 1,
			useColorSpace: false
		}
	};
	return { palette, settings };
}

/** Admit only the S39-proven subset, outside timers. Ordinary TS preparation stays inside each call. */
export function prepareIndexed(request: QuantizeRequest | ProcessRequest): () => IndexedImage {
	const { palette, settings } = websiteRequest(request);
	if (
		!faithfulTypeScriptFallback(
			{ ...request.source, data: new Uint8ClampedArray(request.source.data) },
			palette,
			settings
		)
	)
		throw new Error(
			'No faithful TypeScript comparison for this source, palette, or nearest coordinate map.'
		);
	return () => {
		const { palette, settings } = websiteRequest(request);
		const source = request.source;
		const input = new ImageData(new Uint8ClampedArray(source.data), source.width, source.height);
		const image =
			'recipe' in request
				? resizeImageData(input, settings.output.width, settings.output.height, 'nearest')
				: input;
		const result = quantizeImage(image, palette, settings);
		if (result.warnings.length) throw new Error('Unexpected TypeScript normalization warning.');
		return {
			width: image.width,
			height: image.height,
			indices: result.indices,
			palette: {
				rgba: Uint8Array.from(
					result.palette.flatMap((entry) =>
						entry.rgb ? [entry.rgb.r, entry.rgb.g, entry.rgb.b, 255] : [0, 0, 0, 0]
					)
				),
				transparentIndex: result.transparentIndex
			},
			warnings: []
		};
	};
}
