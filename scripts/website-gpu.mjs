import { spawn } from 'node:child_process';
import { readFile, readdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { relative } from 'node:path';

const crate = new URL('../crates/ditherette-wasm/', import.meta.url);
const assets = new URL('dist/gpu/', crate);
const factoryUrl = "new URL('./wasm/gpu/ditherette_wasm.factory.js', import.meta.url)";
/** @type {Promise<void> | undefined} */
let build;

/**
 * Keep the optional package factory, its imports and Wasm together after worker bundling.
 * @returns {import('vite').Plugin}
 */
export function websiteGpu({ worker = false } = {}) {
	let base = '/';
	let command = 'serve';
	return {
		name: 'ditherette-website-gpu',
		configResolved(config) {
			base = config.base;
			command = config.command;
		},
		async buildStart() {
			if (worker || command !== 'build' || process.env.VITEST) return;
			// The ordinary package still ships CPU artifacts only. Website builds opt in here.
			// pnpm exec supplies this crate's declared wasm-pack binary.
			build ??= new Promise((resolve, reject) => {
				const child = spawn('pnpm', ['exec', process.execPath, 'scripts/build.mjs', 'gpu'], {
					cwd: crate,
					stdio: 'inherit'
				});
				child.once('error', reject);
				child.once('exit', (code) =>
					code === 0 ? resolve() : reject(new Error(`GPU build exited ${code}`))
				);
			});
			await build;
		},
		transform(code, id, options) {
			if (options?.ssr) return;
			if (!id.replaceAll('\\', '/').endsWith('/gpu-effects.js') || !code.includes(factoryUrl))
				return;
			return code.replace(
				factoryUrl,
				`new URL(${JSON.stringify(`${base}wasm/gpu/ditherette_wasm.factory.js`)}, globalThis.location.origin)`
			);
		},
		configureServer(server) {
			server.middlewares.use(async (request, response, next) => {
				const prefix = `${base}wasm/gpu/`;
				if (!request.url?.startsWith(prefix)) return next();
				const relative = request.url.slice(prefix.length).split('?')[0];
				if (!relative || relative.split('/').some((part) => part === '..')) return next();
				try {
					const file = new URL(relative, assets);
					if (!file.href.startsWith(assets.href)) return next();
					const content = await readFile(file);
					response.setHeader(
						'Content-Type',
						relative.endsWith('.wasm') ? 'application/wasm' : 'text/javascript'
					);
					response.end(content);
				} catch {
					next();
				}
			});
		},
		async generateBundle() {
			if (worker) return;
			for (const entry of await readdir(assets, { recursive: true, withFileTypes: true })) {
				if (!entry.isFile() || !/\.(js|wasm)$/.test(entry.name)) continue;
				const file = `${entry.parentPath}/${entry.name}`;
				const path = relative(fileURLToPath(assets), file);
				this.emitFile({
					type: 'asset',
					fileName: `wasm/gpu/${path}`,
					source: await readFile(file)
				});
			}
		}
	};
}
