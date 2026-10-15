import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { installedBrowserChecks } from './installed-browser.mjs';

test('installed package preserves gradient placement across actual scalar/threaded worker routes', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{ publicCases: 14, exactScalarThreaded: true, repeatedPlateaus: true, actualWorkers: true },
		{
			isolated: true,
			assets: Object.fromEntries(
				['smooth-gradient-worker.mjs', 'smooth-gradient-fixture.mjs'].map((name) => [
					`__tests__/${name}`,
					fileURLToPath(new URL(name, import.meta.url))
				])
			),
			driver: async ({ page, server, input, t }) => {
				await page.goto(server.url);
				const result = await page.evaluate(
					(input) =>
						new Promise((resolve, reject) => {
							const worker = new Worker('/__tests__/smooth-gradient-worker.mjs', {
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
							worker.postMessage(input);
						}),
					input
				);
				const { budgetMinima, ...summary } = result;
				t.diagnostic(
					`Threaded minimum bytes (perturb, fused Bayer, resized diffusion): ${budgetMinima?.join(', ')}.`
				);
				return summary;
			}
		}
	));
