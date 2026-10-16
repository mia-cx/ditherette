import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { installedBrowserChecks } from './installed-browser.mjs';

// Proves the installed scalar and threaded packages preserve tolerance inclusion in real workers.
test('installed scalar/threaded selection is monotone at adjacent f32 tolerances', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{ exactScalarThreaded: true, monotoneSelection: true, actualWorkers: true },
		{
			isolated: true,
			assets: Object.fromEntries(
				['smooth-area-monotonic-worker.mjs', 'smooth-area-fixture.mjs'].map((name) => [
					`__tests__/${name}`,
					fileURLToPath(new URL(name, import.meta.url))
				])
			),
			driver: async ({ page, server }) => {
				await page.goto(server.url);
				return page.evaluate(
					() =>
						new Promise((resolve, reject) => {
							const worker = new Worker('/__tests__/smooth-area-monotonic-worker.mjs', {
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
			}
		}
	));

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
