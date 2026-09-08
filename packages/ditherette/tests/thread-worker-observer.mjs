/** Name real Worker instances so a test-only bootstrap prelude can report their lifetime. */
export function observeWorkers(group) {
	const OriginalWorker = globalThis.Worker;
	const workers = [];
	globalThis.Worker = class extends OriginalWorker {
		constructor(url, options) {
			super(url, { ...options, name: `ditherette-test-${group}-${workers.length + 1}` });
			workers.push(this);
		}
	};
	return {
		workers,
		restore() {
			globalThis.Worker = OriginalWorker;
		}
	};
}

/** A held browser lock belongs to the actual worker, not a terminate-call counter. */
export const workerLifetimePrelude = `
const lifetimeMessages = [];
const holdLifetimeMessage = event => {
	lifetimeMessages.push(event);
	event.stopImmediatePropagation();
};
self.addEventListener('message', holdLifetimeMessage);
if (self.name.startsWith('ditherette-test-')) {
	await new Promise((acquired, reject) => {
		navigator.locks.request(self.name, () => {
			acquired();
			return new Promise(() => {});
		}).catch(reject);
	});
	if (self.name.includes('-fail-')) {
		await new Promise(resolve => {
			const gate = new BroadcastChannel(self.name);
			gate.onmessage = () => { gate.close(); resolve(); };
		});
		throw new Error('Injected worker startup failure after actual lock acquisition.');
	}
}
`;

// Lock acquisition yields once. Deliver any early initialization message after the real handlers exist.
export const workerLifetimeEpilogue = `
self.removeEventListener('message', holdLifetimeMessage);
for (const event of lifetimeMessages) self.dispatchEvent(new MessageEvent('message', {
	data: event.data, ports: event.ports, origin: event.origin, lastEventId: event.lastEventId
}));
`;
