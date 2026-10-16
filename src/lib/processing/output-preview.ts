import { drawProcessedImage } from './render';
import type { ProcessedImage } from './types';

type Surface = OffscreenCanvas | HTMLCanvasElement;
/** One zoom level the preview draws: a bitmap from the worker, or a canvas built on the page. */
export type PreviewLevel = ImageBitmap | Surface;

// Keep the exact full-size level for pixel zoom and geometry. Extra levels share a 16 MiB
// RGBA budget, preferring the smallest levels for zoomed-out views. Together with the existing
// 64 MP output cap this bounds retained preview pixels to 256 + 16 MiB. This is a host-memory
// policy, not a processing or export limit. Build skipped intermediates to preserve 2x2 averaging.
const MAX_MIP_BYTES = 16 * 1024 * 1024;

/**
 * Exact full-size output plus a bounded tail of halved levels down to one pixel. Uses
 * `OffscreenCanvas` where it exists so workers can build it; nothing is read back.
 */
export function buildOutputPyramid(image: ProcessedImage): Surface[] {
	const full = canvasFor(image.width, image.height);
	const levels = [full.canvas];
	let current = full.canvas;
	try {
		drawProcessedImage(full.context, image);
		const firstMip = firstRetainedMip(image.width, image.height);
		let mip = 0;
		while (current.width > 1 || current.height > 1) {
			const half = halve(current);
			if (current !== full.canvas && mip < firstMip) releaseLevels([current]);
			current = half;
			if (++mip >= firstMip) levels.push(current);
		}
		return levels;
	} catch (error) {
		releaseLevels(levels);
		if (!levels.includes(current)) releaseLevels([current]);
		throw error;
	}
}

/**
 * The pyramid as bitmaps a worker can transfer, or nothing where a worker can't draw; the page then
 * builds the levels itself.
 */
export function outputPyramidBitmaps(image: ProcessedImage): ImageBitmap[] | undefined {
	if (typeof OffscreenCanvas === 'undefined' || !OffscreenCanvas.prototype.transferToImageBitmap)
		return undefined;
	const levels = buildOutputPyramid(image);
	const bitmaps: ImageBitmap[] = [];
	try {
		for (const level of levels) {
			bitmaps.push((level as OffscreenCanvas).transferToImageBitmap());
			// Transfer replaces the canvas backing store. Discard that empty allocation immediately.
			releaseLevels([level]);
		}
		return bitmaps;
	} catch (error) {
		releaseLevels(bitmaps);
		throw error;
	} finally {
		releaseLevels(levels);
	}
}

/** Free levels' pixels now, including canvases retained by a pending draw callback. */
export function releaseLevels(levels: readonly PreviewLevel[]) {
	for (const level of levels) {
		if ('close' in level) level.close();
		else {
			level.width = 0;
			level.height = 0;
		}
	}
}

/**
 * Draw with the complete pyramid's prefilter, rebuilding omitted levels temporarily.
 * The synchronous callback borrows its level until it returns; retained pixels remain owned by the caller.
 */
export function withOutputPreviewLevel(
	levels: readonly PreviewLevel[],
	targetWidth: number,
	draw: (level: PreviewLevel) => void
) {
	const retained =
		levels.findLast((level) => level.width >= targetWidth) ?? levels[levels.length - 1];
	let current = retained;
	try {
		// Do not scale across a gap in the retained pyramid: high-quality browser smoothing
		// still aliases 1px dithering. Recreate the old 2x2 prefilter before the final draw.
		// Largest temporary level: ceil(retained.width/2) * ceil(retained.height/2) * 4 bytes.
		// A step holds its previous temporary, optional even-size padding, and successor.
		// Each previous/padded canvas is discarded immediately; none survive this draw.
		while (
			(current.width > 1 || current.height > 1) &&
			Math.ceil(current.width / 2) >= targetWidth
		) {
			const next = halve(current);
			if (current !== retained) releaseLevels([current]);
			current = next;
		}
		draw(current);
	} finally {
		if (current !== retained) releaseLevels([current]);
	}
}

function firstRetainedMip(width: number, height: number) {
	const bytes: number[] = [];
	while (width > 1 || height > 1) {
		width = Math.ceil(width / 2);
		height = Math.ceil(height / 2);
		bytes.push(width * height * 4);
	}
	let remaining = MAX_MIP_BYTES;
	let first = bytes.length;
	while (first > 0 && bytes[first - 1] <= remaining) remaining -= bytes[--first];
	return first + 1;
}

function halve(current: PreviewLevel): Surface {
	const source = evenSized(current);
	let half: Surface | undefined;
	try {
		const target = canvasFor(source.width / 2, source.height / 2);
		half = target.canvas;
		// At exactly half size, bilinear sampling averages each 2×2 block in premultiplied alpha.
		target.context.imageSmoothingQuality = 'low';
		target.context.drawImage(source, 0, 0, half.width, half.height);
		return half;
	} catch (error) {
		if (half) releaseLevels([half]);
		throw error;
	} finally {
		if (source !== current) releaseLevels([source]);
	}
}

function canvasFor(width: number, height: number) {
	let allocated: Surface | undefined;
	try {
		if (typeof OffscreenCanvas !== 'undefined') {
			const canvas = new OffscreenCanvas(width, height);
			allocated = canvas;
			return { canvas, context: canvas.getContext('2d') ?? unavailable() };
		}
		const canvas = Object.assign(document.createElement('canvas'), { width, height });
		allocated = canvas;
		return { canvas, context: canvas.getContext('2d') ?? unavailable() };
	} catch (error) {
		if (allocated) releaseLevels([allocated]);
		throw error;
	}
}

function unavailable(): never {
	throw new Error('Could not draw the output preview.');
}

/**
 * Pad an odd side by repeating its last row or column. Bilinear sampling at exactly half size then
 * averages whole 2×2 blocks; at any other scale it can skip thin lines.
 */
function evenSized(level: PreviewLevel): PreviewLevel {
	const width = level.width + (level.width % 2);
	const height = level.height + (level.height % 2);
	if (width === level.width && height === level.height) return level;
	const { canvas, context } = canvasFor(width, height);
	try {
		context.drawImage(level, 0, 0);
		if (width > level.width)
			context.drawImage(
				level,
				level.width - 1,
				0,
				1,
				level.height,
				level.width,
				0,
				1,
				level.height
			);
		if (height > level.height)
			context.drawImage(canvas, 0, level.height - 1, width, 1, 0, level.height, width, 1);
		return canvas;
	} catch (error) {
		releaseLevels([canvas]);
		throw error;
	}
}
