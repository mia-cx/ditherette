import { createDitherette, DitheretteError } from '../src/index.js';
import type {
	Ditherette,
	ResizeRequest,
	Rgba8Image,
	QuantizeRequest,
	IndexedImage,
	PerturbRequest,
	DitherAndQuantizeRequest,
	ProcessRequest
} from '../src/index.js';

const progress: NonNullable<ProcessRequest['onProgress']> = (event) => {
	const stage: import('../src/index.js').Progress['stage'] = event.stage;
	const completed: number | undefined = event.completed;
	const total: number | undefined = event.total;
	void [stage, completed, total];
};

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
	instance.resize({ ...request, onProgress: progress });
	image.data[0] = 255;
	instance.dispose();
	// @ts-expect-error Process settings belong inside the versioned recipe.
	instance.process(request);
	// @ts-expect-error Raw bindings are not public processor state.
	void instance.wasm;
});

const quantize: QuantizeRequest = {
	version: 1,
	source: request.source,
	palette: [{ kind: 'color', rgb: [255, 0, 0] }, { kind: 'transparent' }],
	alpha: { mode: 'preserve', threshold: 127.9999999 },
	matching: 'oklab-euclidean'
};
const complete: ProcessRequest = {
	source: request.source,
	palette: quantize.palette,
	recipe: {
		version: 1,
		output: request.output,
		alpha: quantize.alpha,
		match: quantize.matching,
		dither: { family: 'none' }
	}
};
processor.then((instance) => {
	const image: IndexedImage = instance.process(complete);
	void image;
});
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
void invalidPair;
// @ts-expect-error RGB triples require every byte.
const shortPalette: QuantizeRequest['palette'] = [{ kind: 'color', rgb: [0, 0] }];
void shortPalette;
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
const yliluoma: DitherAndQuantizeRequest['dither'] = {
	family: 'yliluoma',
	size: '16',
	placement: { mode: 'everywhere' }
};
const numericMix: DitherAndQuantizeRequest['dither'] = {
	family: 'yliluoma',
	// @ts-expect-error Yliluoma size is a canonical string tag.
	size: 4,
	placement: { mode: 'everywhere' }
};
const strengthMix: DitherAndQuantizeRequest['dither'] = {
	family: 'yliluoma',
	size: '4',
	placement: { mode: 'everywhere' },
	// @ts-expect-error Yliluoma has no strength control.
	strength: 1
};
void yliluoma;
void numericMix;
void strengthMix;
const effects: import('../src/index.js').Effect[] = [
	{
		effect: 'levels',
		enabled: true,
		channel: 'rgb',
		input: { black: 0.05, white: 0.95 },
		gamma: 1.2,
		output: { black: 0, white: 1 }
	}
];
const graded: ProcessRequest = { ...complete, recipe: { ...complete.recipe, version: 2, effects } };
processor.then((instance) => {
	const image: Rgba8Image = instance.applyEffects({ version: 1, source: request.source, effects });
	const indexed: IndexedImage = instance.process(graded);
	void [image, indexed];
});
// @ts-expect-error Recipe v1 has no effects; use version 2.
const effectsInV1: ProcessRequest['recipe'] = { ...complete.recipe, effects };
// @ts-expect-error Every step states whether it is enabled.
const missingToggle: import('../src/index.js').Effect = { ...effects[0], enabled: undefined };
void [effectsInV1, missingToggle];
const grade: import('../src/index.js').Effect[] = [
	{ effect: 'curves', enabled: true, channel: 'rgb', points: [[0, 0], [0.5, 0.6], [1, 1]] },
	{ effect: 'brightness-contrast', enabled: true, brightness: 0, contrast: 0.2 },
	{ effect: 'exposure', enabled: false, stops: -1 },
	{ effect: 'white-balance', enabled: true, temperature: 0.3, tint: 0 },
	{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: -0.2, lightness: 0 }
];
// @ts-expect-error Curve points are [x, y] pairs.
const flatCurve: import('../src/index.js').CurvesEffect['points'] = [0, 1];
void [grade, flatCurve];
