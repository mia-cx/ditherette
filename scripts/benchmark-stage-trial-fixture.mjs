import { fileURLToPath } from 'node:url';
import { runTrial } from './benchmark-public-page.mjs';
import { events, reset } from './benchmark-stage-cache-fixture.mjs';

/** Exercise the complete warm Process protocol with a fake package and clock, never performance measurements. */
export async function warmProcessTrial() {
	const globals = ['location', 'crossOriginIsolated', 'performance', 'fetch'];
	const previous = globals.map((name) => Object.getOwnPropertyDescriptor(globalThis, name));
	let tick = 0;
	const replacements = [
		{ href: 'file:///' },
		false,
		{ now: () => ++tick },
		async () => new Response(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]))
	];
	for (const [index, name] of globals.entries()) {
		Object.defineProperty(globalThis, name, { configurable: true, value: replacements[index] });
	}
	const dimensions = { width: 1, height: 1 };
	const reference = {
		dimensions,
		pixels: {
			format: 'indexed8',
			indices: [0],
			palette_rgba: [0, 0, 0, 255],
			transparent_index: null
		},
		warnings: []
	};
	const trial = {
		role: 'candidate',
		pair: 0,
		browser: {
			assets: {
				entries: {
					package: fileURLToPath(
						new URL('./benchmark-stage-cache-fixture.mjs', import.meta.url)
					).slice(1),
					wasm: 'unused.wasm'
				}
			}
		},
		reference_output: reference,
		prime_reference_output: {
			dimensions,
			pixels: { format: 'rgba8', data: [1, 2, 3, 255] },
			warnings: []
		},
		case: {
			name: 'warm-process-protocol-fixture',
			source: dimensions,
			rgba: [1, 2, 3, 255],
			identity: { input: Array(32).fill(1), settings: Array(32).fill(2), output: dimensions },
			measurement: {
				mode: 'single-call',
				scope: 'complete-call',
				application_cache: 'warm',
				warmup_ms: 1,
				samples: 5,
				measurement_ms: 10
			},
			browser: {
				operation: {
					operation: 'process',
					settings: {
						palette: [{ kind: 'color', rgb: [0, 0, 0] }],
						recipe: {
							version: 1,
							output: { ...dimensions, resize: { algorithm: 'nearest', anchor: 'center' } },
							alpha: { mode: 'premultiplied' },
							match: 'srgb-euclidean',
							dither: { family: 'none' }
						}
					}
				},
				accepted: 'package',
				candidate: 'package',
				preparation: 'primed-sample',
				cache: {
					roles: { accepted: 'preparation', candidate: 'image-stages', sample_prime: 'resize' }
				}
			}
		}
	};
	try {
		reset();
		const result = await runTrial(trial);
		return { result, events: structuredClone(events), trial };
	} finally {
		for (const [index, name] of globals.entries()) {
			if (previous[index]) Object.defineProperty(globalThis, name, previous[index]);
			else delete globalThis[name];
		}
	}
}
