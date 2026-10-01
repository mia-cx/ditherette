import { processedToImageData } from './render';
import type { ProcessedImage } from './types';

type Surface = OffscreenCanvas | HTMLCanvasElement;
/** One zoom level the preview draws: a bitmap from the worker, or a canvas built on the page. */
export type PreviewLevel = ImageBitmap | Surface;

/**
 * The output at full size, then halved again and again down to one pixel, for drawing the preview
 * at any zoom without aliasing. Uses `OffscreenCanvas` where it exists, so the processing worker
 * can build it; nothing is read back.
 */
export function buildOutputPyramid(image: ProcessedImage): Surface[] {
	const full = canvasFor(image.width, image.height);
	full.context.putImageData(processedToImageData(image), 0, 0);
	const levels = [full.canvas];
	let current = full.canvas;
	while (current.width > 1 || current.height > 1) {
		const source = evenSized(current);
		const half = canvasFor(source.width / 2, source.height / 2);
		// At exactly half size, bilinear sampling averages each 2×2 block in premultiplied alpha.
		half.context.imageSmoothingQuality = 'low';
		half.context.drawImage(source, 0, 0, half.canvas.width, half.canvas.height);
		levels.push(half.canvas);
		current = half.canvas;
	}
	return levels;
}

/**
 * The pyramid as bitmaps a worker can transfer, or nothing where a worker can't draw; the page then
 * builds the levels itself.
 */
export function outputPyramidBitmaps(image: ProcessedImage): ImageBitmap[] | undefined {
	if (typeof OffscreenCanvas === 'undefined' || !OffscreenCanvas.prototype.transferToImageBitmap)
		return undefined;
	return buildOutputPyramid(image).map((level) =>
		(level as OffscreenCanvas).transferToImageBitmap()
	);
}

/** Free levels' pixels now: bitmaps hold theirs until closed. */
export function releaseLevels(levels: readonly PreviewLevel[]) {
	for (const level of levels) if ('close' in level) level.close();
}

function canvasFor(width: number, height: number) {
	if (typeof OffscreenCanvas !== 'undefined') {
		const canvas = new OffscreenCanvas(width, height);
		return { canvas, context: canvas.getContext('2d') ?? unavailable() };
	}
	const canvas = Object.assign(document.createElement('canvas'), { width, height });
	return { canvas, context: canvas.getContext('2d') ?? unavailable() };
}

function unavailable(): never {
	throw new Error('Could not draw the output preview.');
}

/**
 * Pad an odd side by repeating its last row or column. Bilinear sampling at exactly half size then
 * averages whole 2×2 blocks; at any other scale it can skip thin lines.
 */
function evenSized(level: Surface): Surface {
	const width = level.width + (level.width % 2);
	const height = level.height + (level.height % 2);
	if (width === level.width && height === level.height) return level;
	const { canvas, context } = canvasFor(width, height);
	context.drawImage(level, 0, 0);
	if (width > level.width)
		context.drawImage(level, level.width - 1, 0, 1, level.height, level.width, 0, 1, level.height);
	if (height > level.height)
		context.drawImage(canvas, 0, level.height - 1, width, 1, 0, level.height, width, 1);
	return canvas;
}
