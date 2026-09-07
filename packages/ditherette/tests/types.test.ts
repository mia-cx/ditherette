import { createDitherette, DitheretteError } from '../src/index.js';
import type { Ditherette, ResizeRequest, Rgba8Image } from '../src/index.js';

const request: ResizeRequest = {
	version: 1,
	source: { width: 1, height: 1, data: new Uint8Array(4) },
	output: { width: 2, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } }
};
const processor: Promise<Ditherette> = createDitherette({
	threads: 'disabled',
	memoryLimitBytes: 1024
});
processor.then((instance) => {
	const image: Rgba8Image = instance.resize(request);
	image.data[0] = 255;
	instance.dispose();
	// @ts-expect-error The complete pipeline is not exposed before its implementation slice.
	instance.process(request);
	// @ts-expect-error Raw bindings are not public processor state.
	instance.wasm;
});
// @ts-expect-error Noncanonical anchor object tags are not accepted.
const invalidAnchor: Extract<ResizeRequest['output']['resize'], { anchor: unknown }>['anchor'] = {
	center: null
};
void invalidAnchor;
const area: ResizeRequest = {
	...request,
	output: { ...request.output, resize: { algorithm: 'area' } }
};
const bilinear: ResizeRequest = {
	...request,
	output: { ...request.output, resize: { algorithm: 'bilinear', anchor: 'top-left' } }
};
void area;
void bilinear;
const trilinear: ResizeRequest['output']['resize'] = { algorithm: 'trilinear', anchor: 'center' };
const invalidTrilinear: ResizeRequest['output']['resize'] = {
	algorithm: 'trilinear',
	anchor: 'center',
	// @ts-expect-error Trilinear has no support setting.
	support: 'fixed'
};
void trilinear;
void invalidTrilinear;
const convolution: ResizeRequest['output']['resize'][] = [
	{ algorithm: 'bicubic', anchor: 'bottom-right', support: 'fixed' },
	{ algorithm: 'lanczos2', anchor: 'center', support: 'scale-aware' },
	{ algorithm: 'lanczos3', anchor: 'top', support: 'fixed' }
];
// @ts-expect-error Convolution requires an explicit support policy.
const missingSupport: ResizeRequest['output']['resize'] = {
	algorithm: 'bicubic',
	anchor: 'center'
};
const invalidSupport: ResizeRequest['output']['resize'] = {
	algorithm: 'lanczos2',
	anchor: 'center',
	// @ts-expect-error Support uses canonical string tags.
	support: 'auto'
};
void convolution;
void missingSupport;
void invalidSupport;
// @ts-expect-error Area has no anchor.
const invalidArea: ResizeRequest['output']['resize'] = { algorithm: 'area', anchor: 'center' };
const invalidBilinear: ResizeRequest['output']['resize'] = {
	algorithm: 'bilinear',
	anchor: 'center',
	// @ts-expect-error Bilinear has no support setting.
	support: 'fixed'
};
void invalidArea;
void invalidBilinear;
// @ts-expect-error No backend-selection option exists.
createDitherette({ backend: 'scalar' });
// @ts-expect-error Threads must use the named policy, not a boolean.
createDitherette({ threads: true });
const error: Error = new DitheretteError('invalid-image', 'source.data', 'Invalid bytes.');
void error;
