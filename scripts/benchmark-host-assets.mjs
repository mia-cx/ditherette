import { cp, mkdir, readdir } from 'node:fs/promises';
import path from 'node:path';

/** Stage the installed bundle and existing host adapters for untimed browser fixtures. */
export async function stageHostAssets(bundle, root, fixture, wasm) {
	await cp(bundle.package, path.join(root, 'package'), { recursive: true, dereference: true });
	await mkdir(path.join(root, 'scripts'));
	for (const name of [
		'benchmark-host-worker',
		'benchmark-public-page',
		'benchmark-public-timing',
		'benchmark-stage-cache',
		'benchmark-progress',
		'benchmark-row-policy'
	])
		await cp(path.join(bundle.scripts, `${name}.mjs`), path.join(root, `scripts/${name}.mjs`));
	await cp(new URL(`./${fixture}`, import.meta.url), path.join(root, 'scripts', fixture));
	const files = [];
	async function walk(relative = '') {
		for (const entry of await readdir(path.join(root, relative), { withFileTypes: true })) {
			const name = path.posix.join(relative, entry.name);
			if (entry.isDirectory()) await walk(name);
			else files.push({ path: name });
		}
	}
	await walk();
	return {
		tree: { root, files },
		entries: { package: 'package/dist/index.js', wasm }
	};
}
