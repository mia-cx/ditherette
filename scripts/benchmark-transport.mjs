import { fstatSync, statSync } from 'node:fs';

/** Run a browser transport under the Rust owner's lease, including shutdown. */
export async function runBrowserTransport({ startServer, launchBrowserServer, run }) {
	const fd = Number(process.env.DITHERETTE_BENCH_LEASE_FD);
	if (
		process.platform === 'win32' ||
		process.env.DITHERETTE_BENCH_TRANSPORT !== '1' ||
		!Number.isInteger(fd) ||
		fd < 3 ||
		process.env.DITHERETTE_BENCH_QUIET !== '1'
	) {
		throw new Error(
			'Run this transport through ditherette-bench wasm-resize during a coordinator quiet phase.'
		);
	}
	const inherited = fstatSync(fd);
	const expected = statSync('/tmp/ditherette-bench.lock');
	if (!inherited.isFile() || inherited.dev !== expected.dev || inherited.ino !== expected.ino) {
		throw new Error('The browser transport did not inherit the benchmark lease.');
	}

	let server;
	let browserServer;
	let startup = Promise.resolve();
	let closing = false;
	let cleanup;
	const close = () =>
		(cleanup ??= (async () => {
			closing = true;
			// The main path propagates startup errors. Shutdown still closes earlier resources.
			await startup.catch(() => {});
			try {
				// BrowserServer.close waits for Playwright's browser process shutdown.
				await browserServer?.close();
			} finally {
				if (server) {
					server.instance.closeAllConnections();
					await new Promise((resolve, reject) => {
						server.instance.close((error) => (error ? reject(error) : resolve()));
					});
				}
			}
		})());
	const stop = (code) => {
		void close().then(
			() => process.exit(code),
			(error) => {
				console.error(error);
				process.exit(5);
			}
		);
	};
	const signals = new Map([
		['SIGINT', () => stop(130)],
		['SIGTERM', () => stop(143)],
		['SIGHUP', () => stop(129)]
	]);
	for (const [signal, listener] of signals) process.on(signal, listener);
	const parentExited = () => stop(5);
	process.stdin.once('end', parentExited);
	process.stdin.resume();
	try {
		startup = startServer();
		server = await startup;
		if (closing) return;
		startup = launchBrowserServer();
		browserServer = await startup;
		if (closing) return;
		await run({ server, browserServer });
	} finally {
		await close();
		for (const [signal, listener] of signals) process.off(signal, listener);
		process.stdin.off('end', parentExited);
		process.stdin.pause();
	}
}
