import { access, cp, rm } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

/** Stage private generated assets and declarations before the wrapper's TypeScript compilation. */
export async function stageWasm(crate, packageDirectory, license) {
	await access(new URL('dist/scalar/ditherette_wasm.factory.js', crate));
	await access(new URL('dist/scalar/ditherette_wasm.factory.d.ts', crate));
	for (const variant of ['scalar', 'threads']) {
		const destination = new URL(`dist/wasm/${variant}/`, packageDirectory);
		await rm(destination, { recursive: true, force: true });
		await cp(new URL(`dist/${variant}/`, crate), destination, { recursive: true });
	}
	await cp(license, new URL('LICENSE', packageDirectory));
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
	await stageWasm(
		new URL('../../../crates/ditherette-wasm/', import.meta.url),
		new URL('../', import.meta.url),
		new URL('../../../LICENSE', import.meta.url)
	);
}
