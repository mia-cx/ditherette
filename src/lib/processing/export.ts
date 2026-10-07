import { computed } from 'nanostores';
import {
	activePalette,
	colorSpace,
	ditherSettings,
	outputSettings,
	paletteEnabled,
	processedImage,
	processingProgress,
	selectedPalette,
	shareExportData,
	sourceMeta
} from '$lib/stores/app';
import { activeEffectSteps, effectLayers } from '$lib/stores/effects';
import { looksApplied } from '$lib/stores/fit-looks';
import { buildExportEvent, type FitEvent } from '$lib/telemetry/event';
import { measurePaletteFits } from './live-effects';
import { processingIdentityHash } from './hash';
import { downloadIndexedPng } from './png';
import type { ProcessedImage } from './types';

/** Identity of the settings on screen; the processed image matches it once processing catches up. */
const currentHash = computed(
	[
		outputSettings,
		ditherSettings,
		colorSpace,
		activeEffectSteps,
		activePalette,
		selectedPalette,
		sourceMeta
	],
	(output, dither, space, effects, palette, colors, source) =>
		processingIdentityHash({
			output,
			dither,
			colorSpace: space,
			effects,
			paletteName: palette.name,
			paletteSource: palette.source,
			palette: colors,
			source
		})
);

/** True when the processed image is finished and reflects the current settings. */
export const exportable = computed(
	[processedImage, processingProgress, currentHash],
	(image, progress, hash) => Boolean(image && !progress && image.settingsHash === hash)
);

/** Download the processed image as an indexed PNG named after the source. */
export function exportPng() {
	const image = processedImage.get();
	if (!image || !exportable.get()) return;
	downloadIndexedPng(image, exportFilename(image));
	if (shareExportData.get()) {
		// Fire and forget: the export never waits on or surfaces the event.
		void sendExportEvent(image).catch((error: unknown) =>
			console.debug('Export event skipped.', error)
		);
	}
}

/** Post the anonymous export event: settings and colour measurements, never the image. */
async function sendExportEvent(image: ProcessedImage) {
	const effects = activeEffectSteps.get();
	const measured = await measurePaletteFits(effects);
	const applied = looksApplied.get();
	const fits: FitEvent[] = [];
	effectLayers
		.get()
		.filter((layer) => layer.step.enabled)
		.forEach((layer, index) => {
			const step = layer.step;
			if (step.effect !== 'palette-fit') return;
			fits.push({
				step: index,
				look: step.look,
				space: step.space,
				edited: step.curves !== null,
				looksApplied: applied.get(layer.id) ?? [step.look],
				measurements: measured.get(index)?.measurements ?? null
			});
		});
	const event = buildExportEvent({
		output: outputSettings.get(),
		dither: ditherSettings.get(),
		colorSpace: colorSpace.get(),
		palette: activePalette.get(),
		enabled: paletteEnabled.get(),
		effects,
		fits,
		width: image.width,
		height: image.height
	});
	await fetch('/api/events', {
		method: 'POST',
		body: JSON.stringify(event),
		keepalive: true,
		headers: { 'content-type': 'application/json' }
	});
}

function exportFilename(image: ProcessedImage) {
	const original = sourceMeta.get()?.name.replace(/\.[^.]+$/, '') || 'image';
	const safeName =
		original
			.trim()
			.replace(/[^a-z0-9._-]+/gi, '-')
			.replace(/^-+|-+$/g, '') || 'image';
	return `${safeName}-ditherette-${image.width}x${image.height}.png`;
}
