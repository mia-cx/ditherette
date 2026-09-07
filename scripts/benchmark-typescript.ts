import { resizeImageData } from '../src/lib/processing/resize';
import type { ResizeAnchor, Rgba8Image } from '../packages/ditherette/src/types';

/** Developer-only recipes registered by the paired browser protocol. */
interface BenchmarkResizeRequest {
	source: Rgba8Image;
	output: {
		width: number;
		height: number;
		resize: { algorithm: 'area' } | { algorithm: 'nearest' | 'bilinear'; anchor: ResizeAnchor };
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
	const result = resizeImageData(input, output.width, output.height, output.resize.algorithm);
	// The website intentionally aliases identity output. The public comparison must own its bytes.
	const data = result === input ? new Uint8Array(result.data) : new Uint8Array(result.data.buffer);
	return { width: result.width, height: result.height, data };
}
