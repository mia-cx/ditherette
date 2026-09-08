type Builder = import('./wasm/threads/ditherette_wasm.js').wbg_rayon_PoolBuilder;
const startupTimeoutMs = 10_000;

/** Own workers immediately, including handles whose initialization has not finished. */
export class WorkerPool {
	#workers: Worker[] = [];
	#memory: WebAssembly.Memory | undefined;
	#disposed = false;

	async start(module: WebAssembly.Module, memory: WebAssembly.Memory, builder: Builder,
		abandon: (builder: Builder) => void): Promise<void> {
		const cleanups: (() => void)[] = [];
		let url: string | undefined;
		let dispatched = false;
		let succeeded = false;
		try {
			if (this.#disposed) throw new Error('Thread-pool owner is disposed.');
			this.#memory = memory;
			const bootstrap = new URL('./thread-worker.js', import.meta.url).href;
			url = URL.createObjectURL(new Blob([`import ${JSON.stringify(bootstrap)};`], { type: 'text/javascript' }));
			const ready: Promise<Error | undefined>[] = [];
			for (let index = 0; index < builder.numThreads(); index++) {
				const worker = new Worker(url, { type: 'module', name: 'ditherette-pool' });
				this.#workers.push(worker);
				ready.push(new Promise<void>((resolve, reject) => {
					const timeout = setTimeout(() => reject(new Error('Thread startup timed out.')), startupTimeoutMs);
					const onError = () => reject(new Error('Thread startup failed.'));
					const onMessage = ({ data }: MessageEvent) => {
						if (data?.type === 'ditherette-worker-ready') resolve();
						if (data?.type === 'ditherette-worker-error') onError();
					};
					worker.addEventListener('message', onMessage);
					worker.addEventListener('error', onError);
					worker.addEventListener('messageerror', onError);
					cleanups.push(() => {
						clearTimeout(timeout);
						worker.removeEventListener('message', onMessage);
						worker.removeEventListener('error', onError);
						worker.removeEventListener('messageerror', onError);
					});
					worker.postMessage({ type: 'ditherette-worker-init', module, memory });
				}).then(() => undefined, () => new Error('Thread startup failed.')));
			}
			// Each promise already handles rejection if a later constructor aborts this loop.
			const failure = (await Promise.all(ready)).find((result) => result !== undefined);
			if (failure) throw failure;
			if (this.#disposed) throw new Error('Thread-pool owner is disposed.');
			const receiver = builder.receiver();
			for (const worker of this.#workers) {
				dispatched = true;
				worker.postMessage({ type: 'ditherette-worker-start', receiver });
			}
			// Pinned Rayon waits until every worker has consumed the borrowed receiver.
			builder.build();
			succeeded = true;
		} catch (error) {
			this.dispose();
			throw error;
		} finally {
			for (const cleanup of cleanups) cleanup();
			if (url !== undefined) URL.revokeObjectURL(url);
			if (dispatched && !succeeded) abandon(builder);
			else builder.free();
		}
	}

	/** Stop each acquired worker once, then release the shared-memory reference. */
	dispose(): void {
		if (this.#disposed) return;
		this.#disposed = true;
		for (const worker of this.#workers) worker.terminate();
		this.#workers = [];
		this.#memory = undefined;
	}
}
