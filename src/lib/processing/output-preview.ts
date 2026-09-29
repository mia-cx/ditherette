import { processedToImageData } from './render';
import type { ProcessedImage } from './types';

/**
 * The output at full size, then halved again and again down to one pixel, for drawing the preview
 * at any zoom without aliasing. Built on `OffscreenCanvas`, so the processing worker can build it
 * and transfer the bitmaps; the main thread only draws them. Nothing is read back.
 */
export function buildOutputPyramid(image: ProcessedImage): ImageBitmap[] {
	const full = canvasFor(image.width, image.height);
	full.context.putImageData(processedToImageData(image), 0, 0);
	const canvases = [full.canvas];
	let current = full.canvas;
	while (current.width > 1 || current.height > 1) {
		const source = evenSized(current);
		const half = canvasFor(source.width / 2, source.height / 2);
		// At exactly half size, bilinear sampling averages each 2×2 block in premultiplied alpha.
		half.context.imageSmoothingQuality = 'low';
		half.context.drawImage(source, 0, 0, half.canvas.width, half.canvas.height);
		canvases.push(half.canvas);
		current = half.canvas;
	}
	return canvases.map((canvas) => canvas.transferToImageBitmap());
}

function canvasFor(width: number, height: number) {
	const canvas = new OffscreenCanvas(width, height);
	const context = canvas.getContext('2d');
	if (!context) throw new Error('Could not draw the output preview.');
	return { canvas, context };
}

/**
 * Pad an odd side by repeating its last row or column. Bilinear sampling at exactly half size then
 * averages whole 2×2 blocks; at any other scale it can skip thin lines.
 */
function evenSized(level: OffscreenCanvas): OffscreenCanvas {
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
