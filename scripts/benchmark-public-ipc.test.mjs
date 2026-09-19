import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { chromium } from 'playwright';
import { rejectOnPageFailure } from './benchmark-public-browser.mjs';

test('large untimed HTTP exchange fits a bounded Node heap and keeps RPC messages compact', async (t) => {
	const temporary = await mkdtemp(path.join(tmpdir(), 'ditherette-ipc-test-'));
	let browserServer;
	try {
		let request = process.env.DITHERETTE_BENCH_IPC_REQUEST;
		if (!request) {
			request = path.join(temporary, 'request.json');
			const source = { width: 1024, height: 768 };
			await writeFile(
				request,
				JSON.stringify({
					case: {
						source,
						rgba: Array.from({ length: source.width * source.height * 4 }, (_, i) => i % 256),
						identity: { output: { width: 512, height: 384 } },
						measurement: { samples: 100 }
					},
					reference_output: {
						dimensions: { width: 512, height: 384 },
						pixels: {
							format: 'rgba8',
							data: Array.from({ length: 512 * 384 * 4 }, (_, i) => (i + 19) % 256)
						},
						warnings: []
					}
				})
			);
		}
		// Parent owns the browser even if the bounded child fails. No image timing runs.
		browserServer = await chromium.launchServer();
		const child = spawn(
			process.execPath,
			[
				'--max-old-space-size=128',
				fileURLToPath(new URL('./benchmark-public-ipc-fixture.mjs', import.meta.url)),
				'http',
				request,
				browserServer.wsEndpoint()
			],
			{ stdio: ['ignore', 'pipe', 'pipe'] }
		);
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', (chunk) => {
			stdout += chunk;
		});
		child.stderr.on('data', (chunk) => {
			stderr += chunk;
		});
		const outcome = await new Promise((resolve, reject) => {
			child.once('error', reject);
			child.once('exit', (code, signal) => resolve({ code, signal }));
		});
		assert.equal(outcome.code, 0, `${JSON.stringify(outcome)}\n${stderr}`);
		const result = JSON.parse(stdout);
		assert.ok(result.sourceChannels >= 3_145_728);
		assert.ok(result.outputChannels >= 786_432);
		assert.ok(result.maxRpcBytes < 65_536);
		t.diagnostic(JSON.stringify(result));
	} finally {
		await browserServer?.close();
		await rm(temporary, { recursive: true, force: true });
	}
});

test('an actual Chromium renderer crash rejects pending transport control', async () => {
	const browser = await chromium.launch();
	try {
		const page = await browser.newPage();
		const session = await page.context().newCDPSession(page);
		const initialListeners = new Set([...page.listeners('crash'), ...page.listeners('close')]);
		const pending = rejectOnPageFailure(page, () => new Promise(() => {}));
		const crashCommand = session.send('Page.crash').then(
			() => undefined,
			(error) => error.message
		);
		await assert.rejects(pending, /renderer crashed/);
		assert.ok(
			[...page.listeners('crash'), ...page.listeners('close')].every((listener) =>
				initialListeners.has(listener)
			)
		);
		await page.close();
		const commandError = await crashCommand;
		if (commandError !== undefined) assert.match(commandError, /closed|crash/i);
	} finally {
		await browser.close();
	}
});
