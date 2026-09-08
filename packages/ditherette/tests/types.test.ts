import { createDitherette, DitheretteError } from '../src/index.js';
import type {
	Ditherette,
	ResizeRequest,
	Rgba8Image,
	QuantizeRequest,
	IndexedImage,
	PerturbRequest,
	DitherAndQuantizeRequest
} from '../src/index.js';

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

const quantize: QuantizeRequest = {
	version: 1,
	source: request.source,
	palette: [{ kind: 'color', rgb: [255, 0, 0] }, { kind: 'transparent' }],
	alpha: { mode: 'preserve', threshold: 127.9999999 },
	matching: 'oklab-euclidean'
};
processor.then((instance) => {
	const image: IndexedImage = instance.quantize(quantize);
	image.indices[0] = 0;
	const transparent: number | null = image.palette.transparentIndex;
	void transparent;
});
const perceptualQuantize: QuantizeRequest = { ...quantize, matching: 'cielab-ciede2000' };
// @ts-expect-error CIEDE2000 is a CIELAB metric, not an OKLCH metric.
const invalidPair: QuantizeRequest = { ...quantize, matching: 'oklch-ciede2000' };
void perceptualQuantize;
// @ts-expect-error RGB triples require every byte.
const shortPalette: QuantizeRequest['palette'] = [{ kind: 'color', rgb: [0, 0] }];
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
const perturb: PerturbRequest = {
	version: 1,
	source: request.source,
	perturb: {
		field: { algorithm: 'bayer', size: '4' },
		space: 'oklch',
		strength: 0.7,
		placement: { mode: 'adaptive', radius: 1, threshold: 5, softness: 10 }
	}
};
const dither: DitherAndQuantizeRequest = {
	...quantize,
	dither: { family: 'separable', perturb: perturb.perturb }
};
const diffusion: DitherAndQuantizeRequest = {
	...quantize,
	dither: {
		family: 'diffusion',
		kernel: 'atkinson',
		feedback: 'matching',
		strength: 1,
		serpentine: true,
		placement: { mode: 'everywhere' }
	}
};
processor.then((instance) => {
	const rgba: Rgba8Image = instance.perturb(perturb);
	const indexed: IndexedImage = instance.ditherAndQuantize(dither);
	instance.ditherAndQuantize(diffusion);
	void rgba;
	void indexed;
});
// @ts-expect-error Bayer matrix sizes are canonical string tags.
const badSize: PerturbRequest['perturb']['field'] = { algorithm: 'bayer', size: 4 };
const blueNoise: PerturbRequest['perturb']['field'] = { algorithm: 'blue-noise' };
// @ts-expect-error Blue noise uses the fixed tile without a seed control.
const seededBlueNoise: PerturbRequest['perturb']['field'] = { algorithm: 'blue-noise', seed: 0 };
// @ts-expect-error Matching metrics are not reversible working spaces.
const badSpace: PerturbRequest['perturb']['space'] = 'srgb-rec709';
void badSize;
void blueNoise;
void seededBlueNoise;
void badSpace;
