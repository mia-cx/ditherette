import type { Ditherette } from 'ditherette';
import { packageProcessRequest } from './package-adapter';
import type { ColorSpaceId, DitherSettings, EnabledPaletteColor, OutputSettings } from './types';
import { initializePackageProcessor } from './worker-pipeline';

export type DitherPreviewParams = {
	dither: DitherSettings;
	output: OutputSettings;
	palette: readonly EnabledPaletteColor[];
	colorSpace: ColorSpaceId;
};

/** Each preview pixel covers this many CSS pixels, so the pattern stays readable. */
const PIXEL_SCALE = 4;
/** A gray ramp with extra stops in the shadows, so texture is easy to compare at every tone. */
const GRADIENT_STOPS = [
	{ position: 0, gray: 0 },
	{ position: 0.2, gray: 32 },
	{ position: 0.5, gray: 119 },
	{ position: 0.8, gray: 221 },
	{ position: 1, gray: 255 }
];

let processor: Promise<Ditherette> | undefined;

/**
 * Svelte action: draw the preview gradient on `canvas`, dithered by the package with exactly the
 * request processing would send for these settings. Previews are tiny, so they run on the main thread.
 */
export function ditherPreview(canvas: HTMLCanvasElement, params: DitherPreviewParams) {
	let current = params;
	let latest = 0;
	async function draw() {
		const run = ++latest;
		// A canvas in a hidden window has no size yet; the observer draws it once it does.
		if (!canvas.clientWidth) return;
		processor ??= initializePackageProcessor().catch((error: unknown) => {
			processor = undefined;
			throw error;
		});
		try {
			const ditherette = await processor;
			if (run === latest) paint(canvas, ditherette, current);
		} catch (error) {
			console.error('Could not draw the dither preview.', error);
		}
	}
	const observer = new ResizeObserver(() => void draw());
	observer.observe(canvas);
	return {
		update(next: DitherPreviewParams) {
			current = next;
			void draw();
		},
		destroy: () => observer.disconnect()
	};
}

function paint(canvas: HTMLCanvasElement, ditherette: Ditherette, params: DitherPreviewParams) {
	const scale = window.devicePixelRatio || 1;
	const displaySize = Math.max(1, Math.round(canvas.clientWidth * scale));
	const size = Math.max(1, Math.round(canvas.clientWidth / PIXEL_SCALE));
	canvas.width = displaySize;
	canvas.height = displaySize;
	const context = canvas.getContext('2d');
	if (!context) return;
	context.clearRect(0, 0, displaySize, displaySize);
	if (!params.palette.some((color) => color.rgb && color.kind !== 'transparent')) return;

	const { request } = packageProcessRequest(
		gradient(size),
		[...params.palette],
		{
			output: { ...params.output, width: size, height: size, crop: undefined },
			dither: params.dither,
			colorSpace: params.colorSpace,
			effects: []
		},
		{ width: size, height: size }
	);
	const { indices, palette } = ditherette.process(request);
	const image = new ImageData(size, size);
	for (let pixel = 0; pixel < indices.length; pixel++) {
		image.data.set(palette.rgba.subarray(indices[pixel]! * 4, indices[pixel]! * 4 + 4), pixel * 4);
	}
	const pixels = document.createElement('canvas');
	pixels.width = size;
	pixels.height = size;
	pixels.getContext('2d')?.putImageData(image, 0, 0);
	context.imageSmoothingEnabled = false;
	context.drawImage(pixels, 0, 0, displaySize, displaySize);
}

/** The preview image: the gray ramp running diagonally from black at bottom left to white at top right. */
function gradient(size: number): ImageData {
	const image = new ImageData(size, size);
	const extent = Math.max(1, size - 1);
	for (let y = 0; y < size; y++) {
		for (let x = 0; x < size; x++) {
			const gray = rampGray((x / extent + 1 - y / extent) / 2);
			image.data.set([gray, gray, gray, 255], (y * size + x) * 4);
		}
	}
	return image;
}

function rampGray(position: number) {
	const next = GRADIENT_STOPS.findIndex((stop) => stop.position >= position);
	const high = GRADIENT_STOPS[Math.max(1, next)]!;
	const low = GRADIENT_STOPS[Math.max(0, next - 1)]!;
	const amount = (position - low.position) / Math.max(Number.EPSILON, high.position - low.position);
	return Math.round(low.gray + (high.gray - low.gray) * amount);
}
