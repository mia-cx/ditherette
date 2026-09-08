import { DitheretteError } from './errors.js';
import { dimensions, field, object, rgbaBytes, validateQuantize } from './validation.js';

const spaces = ['srgb', 'linear-rgb', 'oklab', 'oklch', 'cielab', 'cielch', 'ycbcr'];
const sizes = ['2', '4', '8', '16'];
const maximumF32 = 3.4028234663852886e38;

function scalar(value: unknown, path: string): number {
	if (typeof value !== 'number' || !Number.isFinite(value) || value < 0 || value > maximumF32)
		throw new DitheretteError('invalid-settings', path, 'Expected a finite nonnegative f32 value.');
	return value;
}

function normalizePolicy(value: unknown, path: string) {
	const policy = object(
		value,
		['field', 'space', 'strength', 'placement'],
		'invalid-settings',
		path
	);
	const inputField = object(
		field(policy, 'field'),
		['algorithm', 'size', 'seed'],
		'invalid-settings',
		`${path}.field`
	);
	const algorithm = field(inputField, 'algorithm');
	let fieldTag: number;
	let parameter: number;
	if (algorithm === 'bayer') {
		if (Object.hasOwn(inputField, 'seed'))
			throw new DitheretteError('invalid-settings', `${path}.field.seed`, 'Bayer has no seed.');
		const size = field(inputField, 'size');
		if (typeof size !== 'string' || !sizes.includes(size))
			throw new DitheretteError(
				'invalid-settings',
				`${path}.field.size`,
				'Expected Bayer size 2, 4, 8, or 16 as a string tag.'
			);
		fieldTag = 0;
		parameter = Number(size);
	} else if (algorithm === 'random') {
		if (Object.hasOwn(inputField, 'size'))
			throw new DitheretteError(
				'invalid-settings',
				`${path}.field.size`,
				'Random has no matrix size.'
			);
		const seed = field(inputField, 'seed');
		if (typeof seed !== 'number' || !Number.isInteger(seed) || seed < 0 || seed > 4_294_967_295)
			throw new DitheretteError(
				'invalid-settings',
				`${path}.field.seed`,
				'Expected an unsigned 32-bit seed.'
			);
		fieldTag = 1;
		parameter = seed;
	} else
		throw new DitheretteError(
			algorithm === 'blue-noise' ? 'unsupported-operation' : 'invalid-settings',
			`${path}.field.algorithm`,
			'This field is not implemented in this package checkpoint.'
		);
	const rawSpace = field(policy, 'space');
	const space = typeof rawSpace === 'string' ? spaces.indexOf(rawSpace) : -1;
	if (space < 0)
		throw new DitheretteError(
			'invalid-settings',
			`${path}.space`,
			'Unknown reversible working space.'
		);
	const strength = scalar(field(policy, 'strength'), `${path}.strength`);
	return {
		field: fieldTag,
		parameter,
		space,
		strength,
		...normalizePlacement(field(policy, 'placement'), path)
	};
}

function normalizePlacement(value: unknown, path: string) {
	const inputPlacement = object(
		value,
		['mode', 'radius', 'threshold', 'softness'],
		'invalid-settings',
		`${path}.placement`
	);
	const mode = field(inputPlacement, 'mode');
	let placement: number;
	let radius = 0,
		threshold = 0,
		softness = 0;
	if (mode === 'everywhere') {
		for (const key of ['radius', 'threshold', 'softness'])
			if (Object.hasOwn(inputPlacement, key))
				throw new DitheretteError(
					'invalid-settings',
					`${path}.placement.${key}`,
					'Everywhere placement has no adaptive controls.'
				);
		placement = 0;
	} else if (mode === 'adaptive') {
		const value = field(inputPlacement, 'radius');
		if (typeof value !== 'number' || !Number.isInteger(value) || value < 1 || value > 32_768)
			throw new DitheretteError(
				'invalid-settings',
				`${path}.placement.radius`,
				'Expected a pixel radius from 1 through 32768.'
			);
		radius = value;
		threshold = scalar(field(inputPlacement, 'threshold'), `${path}.placement.threshold`);
		softness = scalar(field(inputPlacement, 'softness'), `${path}.placement.softness`);
		placement = 1;
	} else
		throw new DitheretteError(
			'invalid-settings',
			`${path}.placement.mode`,
			'Unknown placement mode.'
		);
	return { placement, radius, threshold, softness };
}

function normalizationFailure(error: unknown): never {
	if (error instanceof DitheretteError) throw error;
	throw new DitheretteError(
		error instanceof RangeError ? 'wasm-memory-unavailable' : 'invalid-request',
		'request',
		'Field request could not be normalized.'
	);
}

/** Read raw field settings once while the package's shared active-call guard is held. */
export function validatePerturb(value: unknown) {
	try {
		const request = object(
			value,
			['version', 'source', 'perturb', 'onProgress'],
			'invalid-request',
			'request'
		);
		if (field(request, 'version') !== 1)
			throw new DitheretteError('invalid-request', 'version', 'Unsupported recipe version.');
		const progress = field(request, 'onProgress');
		if (progress !== undefined)
			throw new DitheretteError(
				typeof progress === 'function' ? 'unsupported-operation' : 'invalid-settings',
				'onProgress',
				'Progress callbacks are not implemented in this package checkpoint.'
			);
		const policy = normalizePolicy(field(request, 'perturb'), 'perturb');
		const source = object(
			field(request, 'source'),
			['width', 'height', 'data'],
			'invalid-image',
			'source'
		);
		const size = dimensions(source, 32_768, 'invalid-image', 'source');
		return {
			data: rgbaBytes(field(source, 'data'), size.width * size.height * 4),
			sourceWidth: size.width,
			sourceHeight: size.height,
			...policy
		};
	} catch (error) {
		normalizationFailure(error);
	}
}

/** Reuse complete quantize normalization without reading caller properties twice. */
export function validateDitherAndQuantize(value: unknown) {
	try {
		const request = object(
			value,
			['version', 'source', 'palette', 'alpha', 'matching', 'dither', 'onProgress'],
			'invalid-request',
			'request'
		);
		const dither = object(
			field(request, 'dither'),
			['family', 'perturb', 'size', 'placement'],
			'invalid-settings',
			'dither'
		);
		const family = field(dither, 'family');
		let normalized;
		if (family === 'none') {
			for (const key of ['perturb', 'size', 'placement'])
				if (Object.hasOwn(dither, key))
					throw new DitheretteError(
						'invalid-settings',
						`dither.${key}`,
						'None has no dither controls.'
					);
			normalized = {
				family: 0,
				field: 0,
				parameter: 0,
				space: 0,
				strength: 0,
				placement: 0,
				radius: 0,
				threshold: 0,
				softness: 0
			};
		} else if (family === 'separable') {
			for (const key of ['size', 'placement'])
				if (Object.hasOwn(dither, key))
					throw new DitheretteError(
						'invalid-settings',
						`dither.${key}`,
						'Separable settings belong inside perturb.'
					);
			normalized = { family: 1, ...normalizePolicy(field(dither, 'perturb'), 'dither.perturb') };
		} else if (family === 'yliluoma') {
			if (Object.hasOwn(dither, 'perturb'))
				throw new DitheretteError(
					'invalid-settings',
					'dither.perturb',
					'Yliluoma has no perturb policy.'
				);
			const size = field(dither, 'size');
			if (typeof size !== 'string' || !sizes.includes(size))
				throw new DitheretteError(
					'invalid-settings',
					'dither.size',
					'Expected matrix size 2, 4, 8, or 16 as a string tag.'
				);
			normalized = {
				family: 3,
				field: 0,
				parameter: Number(size),
				space: 0,
				strength: 0,
				...normalizePlacement(field(dither, 'placement'), 'dither')
			};
		} else
			throw new DitheretteError(
				family === 'diffusion' ? 'unsupported-operation' : 'invalid-settings',
				'dither.family',
				'This dither family is not implemented in this package checkpoint.'
			);
		const quantize = validateQuantize({
			version: field(request, 'version'),
			source: field(request, 'source'),
			palette: field(request, 'palette'),
			alpha: field(request, 'alpha'),
			matching: field(request, 'matching'),
			onProgress: field(request, 'onProgress')
		});
		return { ...quantize, dither: normalized };
	} catch (error) {
		normalizationFailure(error);
	}
}
