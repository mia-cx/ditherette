import { env } from '$env/dynamic/private';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
	const response = await resolve(event);
	if (!isTruthyFlag(env['VITE_DITHERETTE_WASM_THREADS'])) return response;

	response.headers.set('Cross-Origin-Opener-Policy', 'same-origin');
	response.headers.set('Cross-Origin-Embedder-Policy', 'require-corp');
	response.headers.set('Cross-Origin-Resource-Policy', 'same-origin');
	return response;
};

function isTruthyFlag(value: unknown) {
	return value === true || value === 'true' || value === '1' || value === 'yes';
}
