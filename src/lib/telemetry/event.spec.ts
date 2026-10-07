import { describe, expect, it } from 'vitest';
import { Schema } from 'effect';
import { buildExportEvent, ExportEvent, type FitEvent } from './event';
import { WPLACE } from '$lib/palette/wplace';
import type { ColorSpaceId, DitherSettings, OutputSettings } from '$lib/processing/types';

const output: OutputSettings = {
	width: 40,
	height: 30,
	resize: 'bilinear',
	crop: { x: 3, y: 1, width: 20, height: 10 },
	alphaMode: 'preserve',
	alphaThreshold: 0,
	matteKey: '#FFFFFF',
	lockAspect: false,
	autoSizeOnUpload: false,
	scaleFactor: 1
};
const dither: DitherSettings = {
	algorithm: 'floyd-steinberg',
	strength: 100,
	placement: 'everywhere',
	placementRadius: 3,
	placementThreshold: 12,
	placementSoftness: 8,
	serpentine: true,
	seed: 1,
	useColorSpace: true
};

const fit: FitEvent = {
	step: 0,
	look: 'vivid',
	space: 'oklab',
	edited: true,
	looksApplied: ['fitted', 'vivid'],
	measurements: {
		toneQuantiles: [0.1, 0.3, 0.5, 0.7, 0.9],
		greyShift: 0.02,
		reach: Array.from({ length: 5 }, () => Array.from({ length: 12 }, () => 0.4))
	}
};

const build = () =>
	buildExportEvent({
		output,
		dither,
		colorSpace: 'oklab' as ColorSpaceId,
		palette: WPLACE,
		enabled: {},
		effects: [
			{
				effect: 'palette-fit',
				enabled: true,
				look: 'vivid',
				space: 'oklab',
				strength: 2,
				curves: null,
				mask: [
					{
						x: { model: 'oklch', channel: 'lightness' },
						points: [
							[0, 0],
							[1, 1]
						]
					}
				]
			}
		],
		fits: [fit],
		width: 40,
		height: 30
	});

describe('buildExportEvent', () => {
	it('assembles the v1 payload and strips the crop and every binary', () => {
		const event = build();
		expect(event.version).toBe(1);
		expect(event.export).toEqual({ width: 40, height: 30, format: 'png' });
		expect(event.settings.output.cropped).toBe(true);
		expect(event.settings.output).not.toHaveProperty('crop');
		expect(event.fits).toEqual([fit]);
		expect(event.settings.palette.colours.every((colour) => colour.enabled)).toBe(true);

		// No image bytes or crop leak anywhere: walk the whole JSON tree.
		const stack: unknown[] = [event];
		while (stack.length) {
			const next = stack.pop();
			expect(next).not.toBeInstanceOf(Uint8Array);
			expect(next).not.toBeInstanceOf(Uint8ClampedArray);
			if (next && typeof next === 'object' && !Array.isArray(next)) {
				for (const [key, value] of Object.entries(next)) {
					expect(key).not.toBe('crop');
					stack.push(value);
				}
			} else if (Array.isArray(next)) {
				stack.push(...next);
			}
		}
	});

	it('round-trips through the shared schema', () => {
		const decoded = Schema.decodeUnknownExit(ExportEvent)(JSON.parse(JSON.stringify(build())));
		expect(decoded._tag).toBe('Success');
	});
});
