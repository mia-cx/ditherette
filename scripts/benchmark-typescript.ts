import { resizeImageData } from '../src/lib/processing/resize';
import type { ResizeRequest, Rgba8Image } from '../packages/ditherette/src/types';

/** Actual website nearest, with a durable result matching the package ownership boundary. */
export function resize(request: ResizeRequest): Rgba8Image {
	if (request.output.resize.anchor !== 'center') {
		throw new Error('The TypeScript nearest implementation supports only center alignment.');
	}
	const { source, output } = request;
	if (!(source.data.buffer instanceof ArrayBuffer))
		throw new Error('Expected unshared RGBA8 input.');
	const input = new ImageData(
		new Uint8ClampedArray(source.data.buffer, source.data.byteOffset, source.data.byteLength),
		source.width,
		source.height
	);
	const result = resizeImageData(input, output.width, output.height, 'nearest');
	// The website intentionally aliases identity output. The public comparison must own its bytes.
	const data = result === input ? new Uint8Array(result.data) : new Uint8Array(result.data.buffer);
	return { width: result.width, height: result.height, data };
}
