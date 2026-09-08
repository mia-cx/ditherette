import { DitheretteError } from './errors.js';
import type { ErrorCode } from './errors.js';
import { field, object, validateResize } from './validation.js';
import { validateDitherAndQuantize } from './validation-fields.js';

/** Settings live inside recipe; source, palette, lifecycle, and result-copy failures do not. */
export function processErrorPath(path: string, code: ErrorCode): string {
	if (path === 'version') return 'recipe.version';
	if (path === 'matching') return 'recipe.match';
	if (path === 'perturb' || path.startsWith('perturb.')) return `recipe.dither.${path}`;
	if (
		path === 'alpha' ||
		path.startsWith('alpha.') ||
		path === 'dither' ||
		path.startsWith('dither.')
	)
		return `recipe.${path}`;
	if (code === 'invalid-settings' && (path === 'output' || path.startsWith('output.')))
		return `recipe.${path}`;
	return path;
}

/** Normalize the recipe once under the instance guard, reusing both staged validators. */
export function validateProcess(value: unknown) {
	try {
		const request = object(
			value,
			['source', 'palette', 'recipe', 'onProgress'],
			'invalid-request',
			'request'
		);
		const recipe = object(
			field(request, 'recipe'),
			['version', 'output', 'alpha', 'match', 'dither'],
			'invalid-settings',
			'recipe'
		);
		const version = field(recipe, 'version');
		if (version !== 1)
			throw new DitheretteError('invalid-request', 'recipe.version', 'Unsupported recipe version.');
		const resized = validateResize({
			version,
			source: field(request, 'source'),
			output: field(recipe, 'output'),
			onProgress: field(request, 'onProgress')
		});
		const quantize = validateDitherAndQuantize({
			version,
			source: { width: resized.sourceWidth, height: resized.sourceHeight, data: resized.data },
			palette: field(request, 'palette'),
			alpha: field(recipe, 'alpha'),
			matching: field(recipe, 'match'),
			dither: field(recipe, 'dither')
		});
		return { ...resized, ...quantize };
	} catch (error) {
		if (error instanceof DitheretteError)
			throw new DitheretteError(
				error.code,
				processErrorPath(error.path, error.code),
				error.message
			);
		throw new DitheretteError(
			error instanceof RangeError ? 'wasm-memory-unavailable' : 'invalid-request',
			'request',
			'Process request could not be normalized.'
		);
	}
}
