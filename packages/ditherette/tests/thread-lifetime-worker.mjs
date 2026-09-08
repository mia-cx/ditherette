// Tiny real workers validate the observer independently of the package runtime.
self.onmessage = async ({ data }) => {
	if (data.child) {
		self.child = new Worker(import.meta.url, { type: 'module', name: `${self.name}-child` });
		self.child.onmessage = () => self.postMessage('ready');
		self.child.postMessage({});
	} else self.postMessage('ready');
};
