import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import test from 'node:test';
import { runBrowserTransport } from './benchmark-transport.mjs';

// This fixture owns a detached Node process in place of Chromium. No measurements run.
function resources({ failLaunch = false, marker } = {}) {
	const server = createServer();
	let child;
	let closed = false;
	return {
		server,
		get child() {
			return child;
		},
		get closed() {
			return closed;
		},
		async startServer() {
			server.listen(0, '127.0.0.1');
			await once(server, 'listening');
			return { instance: server };
		},
		async launchBrowserServer() {
			if (failLaunch) throw new Error('fixture launch failed');
			child = spawn(process.execPath, ['-e', "console.log('READY'); setInterval(() => {}, 1000)"], {
				detached: true,
				stdio: ['ignore', 'pipe', 'inherit']
			});
			await once(child.stdout, 'data');
			return {
				async close() {
					const exit = once(child, 'exit');
					child.kill('SIGTERM');
					await exit;
					closed = true;
					if (marker) await writeFile(marker, 'owned browser exited');
				}
			};
		}
	};
}

if (process.argv.includes('--interrupt-fixture')) {
	const owned = resources({ marker: process.env.DITHERETTE_CLEANUP_MARKER });
	await runBrowserTransport({
		...owned,
		run: async () => {
			console.log('READY');
			await new Promise(() => {});
		}
	});
} else {
	for (const mode of ['success', 'launch failure', 'run failure']) {
		test(`transport closes owned resources after ${mode}`, async () => {
			const owned = resources({ failLaunch: mode === 'launch failure' });
			const run = runBrowserTransport({
				...owned,
				run: async () => {
					if (mode === 'run failure') throw new Error('fixture run failed');
				}
			});
			if (mode === 'success') await run;
			else await assert.rejects(run, /fixture .* failed/);
			assert.equal(owned.server.listening, false);
			if (mode !== 'launch failure') {
				assert.equal(owned.closed, true);
				assert.equal(owned.child.signalCode, 'SIGTERM');
			}
		});
	}
}
