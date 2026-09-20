/** Transfer bulk trial data over HTTP outside the collector, in either declared JavaScript context. */
export async function exchangeTrialData({ requestUrl, resultUrl, pageEntry }) {
	const response = await fetch(requestUrl);
	if (!response.ok) throw new Error(`Trial input failed: ${response.status}`);
	const request = await response.json();
	const { runTrial } = await import(`/${pageEntry}`);
	const result = await runTrial(request);
	const uploaded = await fetch(resultUrl, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(result)
	});
	if (!uploaded.ok) throw new Error(`Trial output failed: ${uploaded.status}`);
	return { posted: true };
}

// The parent owns this host until upload or failure. Package pools close inside runTrial first.
if (
	typeof DedicatedWorkerGlobalScope !== 'undefined' &&
	self instanceof DedicatedWorkerGlobalScope
) {
	self.addEventListener(
		'message',
		async ({ data }) => {
			try {
				self.postMessage(await exchangeTrialData(data));
			} catch (error) {
				self.postMessage({ error: error?.stack ?? String(error) });
			} finally {
				self.close();
			}
		},
		{ once: true }
	);
}
