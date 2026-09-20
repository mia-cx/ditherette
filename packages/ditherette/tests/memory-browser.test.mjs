import test from 'node:test';
import { installedBrowserChecks } from './installed-browser.mjs';
import { memoryBrowserChecks } from './memory-browser-fixture.mjs';

test('installed scalar package bounds dimension preflight and repeated-use memory', (t) =>
	installedBrowserChecks(
		t,
		undefined,
		{
			maximumAxisCalls: 10,
			dimensionRejections: 24,
			budgetRejections: 2,
			warmupCalls: 256,
			repeatedCalls: 512,
			durableOutputs: 2
		},
		{
			async driver({ page, server, input, t }) {
				await page.goto(server.url);
				const { memory, ...checks } = await page.evaluate(memoryBrowserChecks, input);
				t.diagnostic(`Actual scalar Wasm memory observations ${JSON.stringify(memory)}`);
				return checks;
			}
		}
	));
