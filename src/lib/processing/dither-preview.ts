import type { Ditherette } from 'ditherette';
import { packageProcessRequest } from './package-adapter';
import type { ColorSpaceId, DitherSettings, EnabledPaletteColor, OutputSettings } from './types';

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

export type DitherPreviewRequest = { id: number; params: DitherPreviewParams; size: number };
export type DitherPreviewResponse = { id: number } & (
	| { pixels: Uint8ClampedArray }
	| { error: string }
);

/**
 * Previews render in their own worker: a large pattern mix can take the package hundreds of
 * milliseconds even at preview size, which would stall the page if it ran here.
 */
let worker: Worker | undefined;
let nextId = 0;
const answers = new Map<number, (pixels: Uint8ClampedArray | undefined) => void>();

function render(params: DitherPreviewParams, size: number) {
	if (!worker) {
		worker = new Worker(new URL('../workers/dither-preview.worker.ts', import.meta.url), {
			type: 'module'
		});
		worker.onmessage = ({ data }: MessageEvent<DitherPreviewResponse>) => {
			if ('error' in data) console.error('Could not draw the dither preview.', data.error);
			answers.get(data.id)?.('pixels' in data ? data.pixels : undefined);
			answers.delete(data.id);
		};
	}
	const id = ++nextId;
	return new Promise<Uint8ClampedArray | undefined>((resolve) => {
		answers.set(id, resolve);
		worker!.postMessage({ id, params, size } satisfies DitherPreviewRequest);
	});
}

/**
 * Svelte action: draw the preview gradient on `canvas`, dithered by the package with exactly the
 * request processing would send for these settings.
 */
export function ditherPreview(canvas: HTMLCanvasElement, params: DitherPreviewParams) {
	let current = params;
	let latest = 0;
	let destroyed = false;
	async function draw() {
		const run = ++latest;
		// A canvas in a hidden window has no size yet; the observer draws it once it does.
		if (!canvas.clientWidth) return;
		const size = Math.max(1, Math.round(canvas.clientWidth / PIXEL_SCALE));
		const pixels = await render(current, size);
		// Newer settings or a removed card make this result stale.
		if (run === latest && !destroyed) paint(canvas, pixels, size);
	}
	const observer = new ResizeObserver(() => void draw());
	observer.observe(canvas);
	return {
		update(next: DitherPreviewParams) {
			current = next;
			void draw();
		},
		destroy() {
			destroyed = true;
			observer.disconnect();
		}
	};
}

function paint(canvas: HTMLCanvasElement, pixels: Uint8ClampedArray | undefined, size: number) {
	const displaySize = Math.max(1, Math.round(canvas.clientWidth * (window.devicePixelRatio || 1)));
	canvas.width = displaySize;
	canvas.height = displaySize;
	const context = canvas.getContext('2d');
	if (!context) return;
	context.clearRect(0, 0, displaySize, displaySize);
	if (!pixels) return;
	const image = new ImageData(new Uint8ClampedArray(pixels), size, size);
	const scratch = document.createElement('canvas');
	scratch.width = size;
	scratch.height = size;
	scratch.getContext('2d')?.putImageData(image, 0, 0);
	context.imageSmoothingEnabled = false;
	context.drawImage(scratch, 0, 0, displaySize, displaySize);
}

/**
 * The preview gradient dithered by the package, as RGBA pixels. An empty palette gives a blank
 * preview.
 */
export function renderDitherPreview(
	ditherette: Ditherette,
	params: DitherPreviewParams,
	size: number
): Uint8ClampedArray {
	const pixels = new Uint8ClampedArray(size * size * 4);
	if (!params.palette.some((color) => color.rgb && color.kind !== 'transparent')) return pixels;
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
	for (let pixel = 0; pixel < indices.length; pixel++) {
		pixels.set(palette.rgba.subarray(indices[pixel]! * 4, indices[pixel]! * 4 + 4), pixel * 4);
	}
	return pixels;
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
