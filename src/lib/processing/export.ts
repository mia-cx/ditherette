import { computed, onStop } from 'nanostores';
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
	sourceImageData,
	sourceMeta
} from '$lib/stores/app';
import { activeEffectSteps, effectLayers } from '$lib/stores/effects';
import { looksApplied } from '$lib/stores/fit-looks';
import { buildExportEvent, type FitEvent } from '$lib/telemetry/event';
import { measurePaletteFits } from './live-effects';
import { processingIdentityHash } from './hash';
import { downloadPngBlob } from './png';
import { encodeIndexedPngOffThread } from './png-worker';
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

let cancelActiveExport: (() => void) | undefined;

/** Stop a pending export without publishing its file or anonymous event. */
export function cancelPngExport() {
	cancelActiveExport?.();
}

// The last export control unmounting ends the export, including client-side navigation.
onStop(exportable, cancelPngExport);
if (import.meta.hot) import.meta.hot.dispose(cancelPngExport);

/** Download the current indexed result without blocking the page or detaching displayed indices. */
export function exportPng() {
	const image = processedImage.get();
	if (!image || !exportable.get()) return;
	cancelPngExport();
	const controller = new AbortController();
	const source = sourceImageData.get();
	const filename = exportFilename(image);
	const isCurrent = () =>
		processedImage.get() === image &&
		sourceImageData.get() === source &&
		!processingProgress.get() &&
		image.settingsHash === currentHash.get();
	const cancel = () => controller.abort();
	const invalidate = () => {
		if (!isCurrent()) cancel();
	};
	const subscriptions = [
		processedImage.listen(invalidate),
		sourceImageData.listen(invalidate),
		processingProgress.listen(invalidate),
		currentHash.listen(invalidate)
	];
	window.addEventListener('pagehide', cancel, { once: true });
	cancelActiveExport = cancel;
	const completed = encodeIndexedPngOffThread(image, controller.signal)
		.then((blob) => {
			if (controller.signal.aborted || !isCurrent()) return false;
			downloadPngBlob(blob, filename);
			return true;
		})
		.catch((error: unknown) => {
			if (!controller.signal.aborted) console.error('PNG export failed.', error);
			return false;
		})
		.finally(() => {
			for (const unsubscribe of subscriptions) unsubscribe();
			window.removeEventListener('pagehide', cancel);
			if (cancelActiveExport === cancel) cancelActiveExport = undefined;
		});
	if (shareExportData.get()) {
		// Fire and forget: the export never waits on or surfaces the event.
		void sendExportEvent({ width: image.width, height: image.height }, completed).catch(
			(error: unknown) => console.debug('Export event skipped.', error)
		);
	}
}

/** Post the anonymous export event: settings and colour measurements, never the image. */
async function sendExportEvent(
	image: Pick<ProcessedImage, 'width' | 'height'>,
	completed: Promise<boolean>
) {
	// One synchronous snapshot: everything the event reports is read before the measurement
	// await, so a mid-measurement edit can't split the event between two states.
	const effects = activeEffectSteps.get();
	const layers = effectLayers.get();
	const applied = looksApplied.get();
	const output = outputSettings.get();
	const dither = ditherSettings.get();
	const space = colorSpace.get();
	const palette = activePalette.get();
	const enabled = paletteEnabled.get();
	if (!(await completed)) return;
	const measured = await measurePaletteFits(effects);
	const fits: FitEvent[] = [];
	layers
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
		output,
		dither,
		colorSpace: space,
		palette,
		enabled,
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
