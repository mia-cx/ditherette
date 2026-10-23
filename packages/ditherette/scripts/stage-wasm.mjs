import { access, cp, rm } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { basename } from 'node:path';

/** Stage private generated assets and declarations before the wrapper's TypeScript compilation. */
export async function stageWasm(
	crate,
	packageDirectory,
	license,
	variants = ['scalar', 'threads', 'gpu']
) {
	// Reject incomplete builds before replacing any installed variant.
	for (const variant of variants)
		for (const file of [
			'ditherette_wasm.factory.js',
			'ditherette_wasm.factory.d.ts',
			'ditherette_wasm_bg.wasm'
		])
			await access(new URL(`dist/${variant}/${file}`, crate));
	// Explicit CPU-only staging keeps a previous GPU build out of that selection.
	if (!variants.includes('gpu'))
		await rm(new URL('dist/wasm/gpu/', packageDirectory), { recursive: true, force: true });
	for (const variant of variants) {
		const destination = new URL(`dist/wasm/${variant}/`, packageDirectory);
		await rm(destination, { recursive: true, force: true });
		// wasm-pack's generated '*' ignore file would remove every asset from the npm tarball.
		await cp(new URL(`dist/${variant}/`, crate), destination, {
			recursive: true,
			filter: (source) => basename(source) !== '.gitignore'
		});
	}
	await cp(license, new URL('LICENSE', packageDirectory));
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
	const variant = process.argv[2];
	if (variant !== undefined && !['scalar', 'gpu'].includes(variant))
		throw new Error('Expected scalar or gpu staging.');
	const variants = variant ? [variant] : undefined;
	await stageWasm(
		new URL('../../../crates/ditherette-wasm/', import.meta.url),
		new URL('../', import.meta.url),
		new URL('../../../LICENSE', import.meta.url),
		variants
	);
}
