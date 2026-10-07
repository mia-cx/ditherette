import { json, type RequestHandler } from '@sveltejs/kit';
import { Exit, Schema } from 'effect';
import { ExportEvent } from '$lib/telemetry/event';

const MAX_BODY = 64 * 1024;

/** Reads the body once, rejecting anything over the event's byte budget. */
async function readBody(request: Request): Promise<string | undefined> {
	const reader = request.body?.getReader();
	if (!reader) return '';
	const chunks: Uint8Array[] = [];
	let total = 0;
	for (;;) {
		const { done, value } = await reader.read();
		if (done) {
			const bytes = new Uint8Array(total);
			let offset = 0;
			for (const chunk of chunks) {
				bytes.set(chunk, offset);
				offset += chunk.length;
			}
			return new TextDecoder().decode(bytes);
		}
		total += value.length;
		if (total > MAX_BODY) {
			await reader.cancel();
			return undefined;
		}
		chunks.push(value);
	}
}

/** Plain `vite dev` has no D1; say so once instead of per export. */
let warned = false;

export const POST: RequestHandler = async ({ request, platform }) => {
	const body = await readBody(request);
	if (body === undefined) return json({ error: 'Payload too large.' }, { status: 413 });
	let parsed: unknown;
	try {
		parsed = JSON.parse(body);
	} catch {
		return json({ error: 'Invalid event.' }, { status: 400 });
	}
	const decoded = Schema.decodeUnknownExit(ExportEvent)(parsed);
	if (Exit.isFailure(decoded)) return json({ error: 'Invalid event.' }, { status: 400 });
	const events = platform?.env?.EVENTS;
	if (!events) {
		if (!warned) {
			warned = true;
			console.warn('Export events need a D1 EVENTS binding; skipping storage.');
		}
		return new Response(null, { status: 204 });
	}
	await events
		.prepare('INSERT INTO export_events (received_at, version, payload) VALUES (?, ?, ?)')
		.bind(new Date().toISOString(), decoded.value.version, JSON.stringify(decoded.value))
		.run();
	return new Response(null, { status: 204 });
};
