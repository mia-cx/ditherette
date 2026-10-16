import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { installedBrowserChecks } from './installed-browser.mjs';

test('installed package preserves complete area footprints across scalar/threaded worker routes', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{
			publicCases: 54,
			exactScalarThreaded: true,
			completeFootprints: true,
			actualWorkers: true
		},
		{
			isolated: true,
			assets: Object.fromEntries(
				['smooth-area-worker.mjs', 'smooth-area-fixture.mjs'].map((name) => [
					`__tests__/${name}`,
					fileURLToPath(new URL(name, import.meta.url))
				])
			),
			driver: async ({ page, server, t }) => {
				await page.goto(server.url);
				const result = await page.evaluate(
					() =>
						new Promise((resolve, reject) => {
							const worker = new Worker('/__tests__/smooth-area-worker.mjs', {
								type: 'module'
							});
							worker.onerror = (event) => {
								worker.terminate();
								reject(new Error(event.message));
							};
							worker.onmessage = ({ data }) => {
								worker.terminate();
								resolve(data);
							};
						})
				);
				const { budgetMinima, ...summary } = result;
				t.diagnostic(
					`Threaded minimum bytes (perturb, fused Bayer, resized diffusion): ${budgetMinima?.join(', ')}.`
				);
				return summary;
			}
		}
	));
