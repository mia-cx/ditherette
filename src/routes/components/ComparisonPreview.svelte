<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import {
		Empty,
		EmptyContent,
		EmptyDescription,
		EmptyHeader,
		EmptyMedia,
		EmptyTitle
	} from '$lib/components/ui/empty';
	import { processedToImageData } from '$lib/processing/render';
	import type { CropRect } from '$lib/processing/types';
	import {
		outputSettings,
		previewSettings,
		processedImage,
		processingError,
		processingProgress,
		sourceImageData,
		sourceMeta,
		sourceObjectUrl,
		updateOutputSettings,
		updatePreviewSettings,
		type PreviewMode
	} from '$lib/stores/app';
	import CropIcon from 'phosphor-svelte/lib/Crop';
	import ArrowsOutIcon from 'phosphor-svelte/lib/ArrowsOut';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlass';
	import ImageIcon from 'phosphor-svelte/lib/ImageSquare';
	import UploadIcon from 'phosphor-svelte/lib/UploadSimple';

	type Point = { x: number; y: number };
	type ViewAnchor = { centerX: number; centerY: number };
	type CropHandle = 'n' | 'e' | 's' | 'w' | 'nw' | 'ne' | 'se' | 'sw';
	type PreviewLevel = { canvas: HTMLCanvasElement; width: number; height: number };

	type Props = {
		defaultMode?: PreviewMode;
		hasImage?: boolean;
		minHeightClass?: string;
		onChooseImage?: () => void;
		onSelectFile?: (file: File) => void;
	};

	let {
		defaultMode = 'side-by-side',
		hasImage = false,
		minHeightClass = 'min-h-[320px] md:min-h-[420px]',
		onChooseImage,
		onSelectFile
	}: Props = $props();

	const MIN_ZOOM = 0.0625;
	const MAX_ZOOM = 256;
	const CROP_HANDLES: CropHandle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];

	const initialPreviewSettings = previewSettings.get();
	// svelte-ignore state_referenced_locally
	let mode = $state<PreviewMode>(initialPreviewSettings.mode ?? defaultMode);
	let revealValue = $state<number>(initialPreviewSettings.revealValue ?? 50);
	let revealDrag = $state<number>();
	let zoom = $state(clampZoom(safeNumber(initialPreviewSettings.zoom, 1)));
	let panX = $state(safeNumber(initialPreviewSettings.panX, 0));
	let panY = $state(safeNumber(initialPreviewSettings.panY, 0));
	let cropMode = $state(false);
	let cropDraft = $state<CropRect>();
	let cropResize = $state<{
		pointerId: number;
		handle: CropHandle;
		start: Point;
		crop: CropRect;
	}>();
	let cropPane = $state<HTMLElement>();
	let pointers = $state<Record<number, Point>>({});
	let pinch = $state<{ distance: number; zoom: number }>();
	let drag = $state<{
		pointerId: number;
		startX: number;
		startY: number;
		panX: number;
		panY: number;
	}>();
	let sideSourcePane = $state<HTMLElement>();
	let sideOutputPane = $state<HTMLElement>();
	let revealPane = $state<HTMLElement>();
	let sideOutputCanvas = $state<HTMLCanvasElement>();
	let revealOutputCanvas = $state<HTMLCanvasElement>();
	let outputPreviewLevels = $state<PreviewLevel[]>([]);
	let outputPreviewGeneration = $state(0);
	let nextOutputPreviewGeneration = 0;
	let lockedFrameWidth = $state<number | undefined>(
		safeOptionalNumber(initialPreviewSettings.frameWidth)
	);
	let lockedFrameHeight = $state<number | undefined>(
		safeOptionalNumber(initialPreviewSettings.frameHeight)
	);
	let viewOriginX = $state(safeNumber(initialPreviewSettings.originX, 0));
	let viewOriginY = $state(safeNumber(initialPreviewSettings.originY, 0));
	let layoutVersion = $state(0);

	const zoomLabel = $derived(`${Math.round(zoom * 100)}%`);
	const sizeLabel = $derived(
		$processedImage
			? `${$processedImage.width} × ${$processedImage.height}`
			: $sourceMeta
				? `${$sourceMeta.width} × ${$sourceMeta.height}`
				: '—'
	);
	const colorLabel = $derived(
		$processedImage ? `${$processedImage.palette.length} colors` : 'Palette'
	);
	const activeCrop = $derived(
		cropDraft ?? $outputSettings.crop ?? (cropMode ? fullImageCrop() : undefined)
	);
	const cropToContentBounds = $derived.by(() => findContentCrop($sourceImageData));
	const canCropToContent = $derived(Boolean(cropToContentBounds));
	const cropToContentHint = $derived(
		canCropToContent ? 'Crop to non-transparent content.' : 'No transparent bounds found to crop.'
	);

	$effect(() => {
		outputPreviewGeneration = ++nextOutputPreviewGeneration;
		if (!$processedImage) {
			outputPreviewLevels = [];
			return;
		}
		outputPreviewLevels = buildPreviewPyramid(processedToImageData($processedImage));
	});

	$effect(() => {
		const levels = outputPreviewLevels;
		const redrawKey = `${mode}:${zoom}:${layoutVersion}:${outputPreviewGeneration}`;
		const sideCanvas = sideOutputCanvas;
		const sidePane = sideOutputPane;
		const revealCanvas = revealOutputCanvas;
		const pane = revealPane;
		if (!levels.length || !redrawKey) return;
		const frame = requestAnimationFrame(() => {
			if (sideCanvas && sidePane)
				renderPreviewCanvas(sideCanvas, sidePane, levels, outputPreviewGeneration);
			if (revealCanvas && pane)
				renderPreviewCanvas(revealCanvas, pane, levels, outputPreviewGeneration);
		});
		return () => cancelAnimationFrame(frame);
	});

	$effect(() =>
		previewSettings.subscribe((settings) => {
			mode = settings.mode ?? defaultMode;
			revealValue = settings.revealValue ?? 50;
			zoom = clampZoom(safeNumber(settings.zoom, 1));
			panX = safeNumber(settings.panX, 0);
			panY = safeNumber(settings.panY, 0);
			lockedFrameWidth = safeOptionalNumber(settings.frameWidth);
			lockedFrameHeight = safeOptionalNumber(settings.frameHeight);
			viewOriginX = safeNumber(settings.originX, 0);
			viewOriginY = safeNumber(settings.originY, 0);
		})
	);

	$effect(() => {
		updatePreviewSettings({
			mode,
			zoom,
			panX,
			panY,
			frameWidth: lockedFrameWidth,
			frameHeight: lockedFrameHeight,
			originX: viewOriginX,
			originY: viewOriginY
		});
	});

	onMount(() => {
		window.addEventListener('resize', updateLayout);
		window.addEventListener('pointerup', cancelPointerInteraction);
		window.addEventListener('pointercancel', cancelPointerInteraction);
		window.addEventListener('blur', cancelPointerInteraction);
		return () => {
			window.removeEventListener('resize', updateLayout);
			window.removeEventListener('pointerup', cancelPointerInteraction);
			window.removeEventListener('pointercancel', cancelPointerInteraction);
			window.removeEventListener('blur', cancelPointerInteraction);
		};
	});

	$effect(() => {
		const observer = new ResizeObserver(updateLayout);
		if (cropPane) observer.observe(cropPane);
		if (sideSourcePane) observer.observe(sideSourcePane);
		if (sideOutputPane) observer.observe(sideOutputPane);
		if (revealPane) observer.observe(revealPane);
		return () => observer.disconnect();
	});

	function safeNumber(value: number | undefined, fallback: number) {
		return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
	}

	function safeOptionalNumber(value: number | undefined) {
		return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
	}

	function pixelRatio() {
		return typeof window === 'undefined' ? 1 : window.devicePixelRatio || 1;
	}

	function clampZoom(value: number) {
		return Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, value || 1));
	}

	function resetView() {
		const pane = activePreviewPane();
		const size = pane ? paneMediaSize() : undefined;
		panX = 0;
		panY = 0;
		if (pane && size) setFittedView(pane, size.width, size.height);
		else clearLockedFrame();
	}

	function cancelPointerInteraction() {
		if (cropResize && cropDraft) cropDraft = normalizeCrop(cropDraft);
		pointers = {};
		pinch = undefined;
		cropResize = undefined;
		drag = undefined;
	}

	function updateLayout() {
		const pane = activePreviewPane();
		const size = pane ? paneMediaSize() : undefined;
		if (pane && size && lockedFrameWidth === undefined)
			setFittedView(pane, size.width, size.height);
		layoutVersion++;
	}

	function setFittedView(pane: HTMLElement, width: number, height: number) {
		zoom = clampZoom(baseScale(pane, width, height) * pixelRatio());
		const frame = zoomFrameSize(width, height);
		lockedFrameWidth = frame.width;
		lockedFrameHeight = frame.height;
		viewOriginX = (pane.clientWidth - frame.width) / 2;
		viewOriginY = (pane.clientHeight - frame.height) / 2;
	}

	function clearLockedFrame() {
		lockedFrameWidth = undefined;
		lockedFrameHeight = undefined;
	}

	function activePreviewPane() {
		if (cropMode) return cropPane;
		if (mode === 'side-by-side') return sideOutputPane ?? sideSourcePane;
		return revealPane;
	}

	function sourceDisplaySize() {
		if (!$sourceMeta) return undefined;
		return { width: $sourceMeta.width, height: $sourceMeta.height };
	}

	function paneMediaSize() {
		const sourceSize = sourceDisplaySize();
		if (sourceSize) return sourceSize;
		if ($processedImage) return { width: $processedImage.width, height: $processedImage.height };
		return undefined;
	}

	function baseScale(pane: HTMLElement, width: number, height: number) {
		return Math.min(pane.clientWidth / width, pane.clientHeight / height);
	}

	function zoomFrameSize(width: number, height: number) {
		const reference = sourceDisplaySize() ?? { width, height };
		const cssScale = clampZoom(zoom) / pixelRatio();
		const frameHeight = reference.height * cssScale;
		return { width: frameHeight * (width / height), height: frameHeight };
	}

	function fitFrame(pane: HTMLElement | undefined, width: number, height: number) {
		if (!pane) return { left: 0, top: 0, width: 0, height: 0 };
		const frame = zoomFrameSize(width, height);
		if (lockedFrameWidth !== undefined && lockedFrameHeight !== undefined) {
			return {
				left: viewOriginX + panX,
				top: viewOriginY + panY,
				width: frame.width,
				height: frame.height
			};
		}
		return {
			left: (pane.clientWidth - frame.width) / 2 + panX,
			top: (pane.clientHeight - frame.height) / 2 + panY,
			width: frame.width,
			height: frame.height
		};
	}

	function currentViewAnchor(): ViewAnchor | undefined {
		const pane = activePreviewPane();
		const size = pane ? paneMediaSize() : undefined;
		if (!pane || !size) return undefined;
		const frame = fitFrame(pane, size.width, size.height);
		if (!frame.width || !frame.height) return undefined;
		return {
			centerX: (pane.clientWidth / 2 - frame.left) / frame.width,
			centerY: (pane.clientHeight / 2 - frame.top) / frame.height
		};
	}

	function applyViewAnchor(anchor: ViewAnchor | undefined) {
		const pane = activePreviewPane();
		const size = pane ? paneMediaSize() : undefined;
		if (!anchor || !pane || !size) return;
		const frame = zoomFrameSize(size.width, size.height);
		lockedFrameWidth = frame.width;
		lockedFrameHeight = frame.height;
		viewOriginX = pane.clientWidth / 2 - anchor.centerX * frame.width;
		viewOriginY = pane.clientHeight / 2 - anchor.centerY * frame.height;
		panX = 0;
		panY = 0;
		layoutVersion++;
	}

	function frameStyle(
		frame: { left: number; top: number; width: number; height: number },
		width: number
	) {
		const rendering = frame.width / width >= 1 ? 'pixelated' : 'auto';
		return `left:${frame.left}px;top:${frame.top}px;width:${frame.width}px;height:${frame.height}px;image-rendering:${rendering};--preview-layout:${layoutVersion}`;
	}

	function mediaStyle(
		pane: HTMLElement | undefined,
		width: number | undefined,
		height: number | undefined
	) {
		if (!width || !height) return '';
		return frameStyle(fitFrame(pane, width, height), width);
	}

	function appliedCrop() {
		return cropMode ? undefined : $outputSettings.crop;
	}

	function cropFrame(pane: HTMLElement | undefined, crop: CropRect) {
		if (!pane || !$sourceMeta) return { left: 0, top: 0, width: 0, height: 0 };
		const frame = fitFrame(pane, $sourceMeta.width, $sourceMeta.height);
		return {
			left: frame.left + (crop.x / $sourceMeta.width) * frame.width,
			top: frame.top + (crop.y / $sourceMeta.height) * frame.height,
			width: (crop.width / $sourceMeta.width) * frame.width,
			height: (crop.height / $sourceMeta.height) * frame.height
		};
	}

	function outputFrame(pane: HTMLElement | undefined, width: number, height: number) {
		const crop = appliedCrop();
		if (crop) return cropFrame(pane, crop);
		return fitFrame(pane, width, height);
	}

	function outputMediaStyle(
		pane: HTMLElement | undefined,
		width: number | undefined,
		height: number | undefined
	) {
		if (!width || !height) return '';
		return frameStyle(outputFrame(pane, width, height), width);
	}

	function buildPreviewPyramid(imageData: ImageData): PreviewLevel[] {
		const levels: PreviewLevel[] = [imageDataToCanvas(imageData)];
		let current = imageData;
		while (current.width > 1 || current.height > 1) {
			const width = Math.max(1, Math.floor(current.width / 2));
			const height = Math.max(1, Math.floor(current.height / 2));
			const canvas = document.createElement('canvas');
			canvas.width = width;
			canvas.height = height;
			drawBoxDownsample(canvas, current, width, height);
			levels.push({ canvas, width, height });
			const context = canvas.getContext('2d');
			if (!context) break;
			current = context.getImageData(0, 0, width, height);
		}
		return levels;
	}

	function imageDataToCanvas(imageData: ImageData): PreviewLevel {
		const canvas = document.createElement('canvas');
		canvas.width = imageData.width;
		canvas.height = imageData.height;
		const context = canvas.getContext('2d');
		context?.putImageData(imageData, 0, 0);
		return { canvas, width: imageData.width, height: imageData.height };
	}

	function renderPreviewCanvas(
		canvas: HTMLCanvasElement,
		pane: HTMLElement,
		levels: PreviewLevel[],
		generation: number
	) {
		const original = levels[0]!;
		const frame = outputFrame(pane, original.width, original.height);
		const cssWidth = Math.max(1, frame.width);
		const cssHeight = Math.max(1, frame.height);
		const deviceScale = window.devicePixelRatio || 1;
		const deviceWidth = Math.max(1, Math.round(cssWidth * deviceScale));
		const deviceHeight = Math.max(1, Math.round(cssHeight * deviceScale));
		const outputScale = deviceWidth / original.width;
		const context = canvas.getContext('2d');
		if (!context) return;

		if (outputScale >= 1) {
			drawPreviewLevel(
				canvas,
				context,
				original,
				original.width,
				original.height,
				false,
				generation
			);
			canvas.style.imageRendering = 'pixelated';
			return;
		}

		const level = selectPreviewLevel(levels, deviceWidth);
		const smoothing = level.width > deviceWidth || level.height > deviceHeight;
		drawPreviewLevel(canvas, context, level, deviceWidth, deviceHeight, smoothing, generation);
		canvas.style.imageRendering = 'auto';
	}

	function selectPreviewLevel(levels: PreviewLevel[], targetWidth: number) {
		for (let index = levels.length - 1; index >= 0; index--) {
			const level = levels[index]!;
			if (level.width >= targetWidth) return level;
		}
		return levels[levels.length - 1]!;
	}

	function drawPreviewLevel(
		canvas: HTMLCanvasElement,
		context: CanvasRenderingContext2D,
		level: PreviewLevel,
		width: number,
		height: number,
		smoothing: boolean,
		generation: number
	) {
		const cacheKey = `${generation}:${level.width}x${level.height}:${width}x${height}:${smoothing}`;
		if (canvas.dataset.previewKey === cacheKey) return;
		if (canvas.width !== width || canvas.height !== height) {
			canvas.width = width;
			canvas.height = height;
		}
		context.imageSmoothingEnabled = smoothing;
		context.imageSmoothingQuality = 'high';
		context.clearRect(0, 0, width, height);
		context.drawImage(level.canvas, 0, 0, width, height);
		canvas.dataset.previewKey = cacheKey;
	}

	function drawBoxDownsample(
		canvas: HTMLCanvasElement,
		image: ImageData,
		width: number,
		height: number
	) {
		const context = canvas.getContext('2d');
		if (!context) return;
		const output = context.createImageData(width, height);
		const source = image.data;
		const target = output.data;
		const xRatio = image.width / width;
		const yRatio = image.height / height;

		for (let y = 0; y < height; y++) {
			const startY = Math.floor(y * yRatio);
			const endY = Math.max(startY + 1, Math.ceil((y + 1) * yRatio));
			for (let x = 0; x < width; x++) {
				const startX = Math.floor(x * xRatio);
				const endX = Math.max(startX + 1, Math.ceil((x + 1) * xRatio));
				let red = 0;
				let green = 0;
				let blue = 0;
				let alpha = 0;
				let samples = 0;

				for (let sourceY = startY; sourceY < endY && sourceY < image.height; sourceY++) {
					let offset = (sourceY * image.width + startX) * 4;
					for (let sourceX = startX; sourceX < endX && sourceX < image.width; sourceX++) {
						const sampleAlpha = source[offset + 3]! / 255;
						red += source[offset]! * sampleAlpha;
						green += source[offset + 1]! * sampleAlpha;
						blue += source[offset + 2]! * sampleAlpha;
						alpha += sampleAlpha;
						samples++;
						offset += 4;
					}
				}

				const offset = (y * width + x) * 4;
				const divisor = alpha || samples || 1;
				target[offset] = red / divisor;
				target[offset + 1] = green / divisor;
				target[offset + 2] = blue / divisor;
				target[offset + 3] = (alpha / Math.max(1, samples)) * 255;
			}
		}

		context.putImageData(output, 0, 0);
	}

	function cropStyle(pane: HTMLElement | undefined, crop: CropRect | undefined) {
		if (!crop || !$sourceMeta) return '';
		const frame = cropFrame(pane, crop);
		return [
			`left:${frame.left}px`,
			`top:${frame.top}px`,
			`width:${frame.width}px`,
			`height:${frame.height}px`,
			`--preview-layout:${layoutVersion}`
		].join(';');
	}

	function cropHandleClass(handle: CropHandle) {
		const positions: Record<CropHandle, string> = {
			n: 'top-0 left-1/2 -translate-x-1/2 -translate-y-1/2 cursor-ns-resize',
			e: 'top-1/2 right-0 translate-x-1/2 -translate-y-1/2 cursor-ew-resize',
			s: 'bottom-0 left-1/2 -translate-x-1/2 translate-y-1/2 cursor-ns-resize',
			w: 'top-1/2 left-0 -translate-x-1/2 -translate-y-1/2 cursor-ew-resize',
			nw: 'top-0 left-0 -translate-x-1/2 -translate-y-1/2 cursor-nwse-resize',
			ne: 'top-0 right-0 translate-x-1/2 -translate-y-1/2 cursor-nesw-resize',
			se: 'right-0 bottom-0 translate-x-1/2 translate-y-1/2 cursor-nwse-resize',
			sw: 'bottom-0 left-0 -translate-x-1/2 translate-y-1/2 cursor-nesw-resize'
		};
		return `pointer-events-auto absolute size-3 border border-background bg-primary shadow ${positions[handle]}`;
	}

	function panePointToImage(
		pane: HTMLElement,
		clientX: number,
		clientY: number
	): Point | undefined {
		if (!$sourceMeta) return undefined;
		const bounds = pane.getBoundingClientRect();
		const frame = fitFrame(pane, $sourceMeta.width, $sourceMeta.height);
		const x = ((clientX - bounds.left - frame.left) / frame.width) * $sourceMeta.width;
		const y = ((clientY - bounds.top - frame.top) / frame.height) * $sourceMeta.height;
		return {
			x: Math.min($sourceMeta.width, Math.max(0, x)),
			y: Math.min($sourceMeta.height, Math.max(0, y))
		};
	}

	function fullImageCrop(): CropRect {
		return { x: 0, y: 0, width: $sourceMeta?.width ?? 1, height: $sourceMeta?.height ?? 1 };
	}

	function resizeCropFromHandle(handle: CropHandle, start: Point, end: Point, crop: CropRect) {
		const deltaX = end.x - start.x;
		const deltaY = end.y - start.y;
		const left = handle.includes('w') ? crop.x + deltaX : crop.x;
		const top = handle.includes('n') ? crop.y + deltaY : crop.y;
		const right = handle.includes('e') ? crop.x + crop.width + deltaX : crop.x + crop.width;
		const bottom = handle.includes('s') ? crop.y + crop.height + deltaY : crop.y + crop.height;
		return normalizeCrop({
			x: Math.min(left, right - 1),
			y: Math.min(top, bottom - 1),
			width: Math.max(1, right - left),
			height: Math.max(1, bottom - top)
		});
	}

	function onPointerDown(event: PointerEvent, pane: HTMLElement | undefined) {
		if (!hasImage || !pane || event.button !== 0) return;
		event.preventDefault();
		pane.setPointerCapture(event.pointerId);
		pointers = { ...pointers, [event.pointerId]: { x: event.clientX, y: event.clientY } };

		if (Object.keys(pointers).length === 2) {
			pinch = { distance: pointerDistance(), zoom };
			drag = undefined;
			cropResize = undefined;
			return;
		}

		drag = { pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, panX, panY };
	}

	function onPointerMove(event: PointerEvent, pane: HTMLElement | undefined) {
		if (event.pointerType === 'mouse' && event.buttons === 0) {
			cancelPointerInteraction();
			return;
		}
		if (pointers[event.pointerId]) {
			pointers = { ...pointers, [event.pointerId]: { x: event.clientX, y: event.clientY } };
		}
		if (pinch && pane && Object.keys(pointers).length >= 2) {
			const distance = pointerDistance();
			const midpoint = pointerMidpoint();
			if (distance > 0 && midpoint) {
				applyZoomAt(
					pane,
					midpoint.x,
					midpoint.y,
					clampZoom(pinch.zoom * (distance / pinch.distance))
				);
			}
			return;
		}
		if (cropResize && pane && cropResize.pointerId === event.pointerId) {
			const end = panePointToImage(pane, event.clientX, event.clientY);
			if (end)
				cropDraft = resizeCropFromHandle(cropResize.handle, cropResize.start, end, cropResize.crop);
			return;
		}
		if (!drag || drag.pointerId !== event.pointerId) return;
		panX = drag.panX + event.clientX - drag.startX;
		panY = drag.panY + event.clientY - drag.startY;
	}

	function onPointerUp(event: PointerEvent) {
		cancelPointerInteraction();
		if (
			event.currentTarget instanceof HTMLElement &&
			event.currentTarget.hasPointerCapture(event.pointerId)
		) {
			event.currentTarget.releasePointerCapture(event.pointerId);
		}
	}

	function onCropHandlePointerDown(
		event: PointerEvent,
		handle: CropHandle,
		pane: HTMLElement | undefined
	) {
		if (!hasImage || !pane || event.button !== 0) return;
		const start = panePointToImage(pane, event.clientX, event.clientY);
		if (!start) return;
		event.preventDefault();
		event.stopPropagation();
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		pointers = { [event.pointerId]: { x: event.clientX, y: event.clientY } };
		pinch = undefined;
		drag = undefined;
		cropResize = { pointerId: event.pointerId, handle, start, crop: editableCrop() };
	}

	function onWheel(event: WheelEvent, pane: HTMLElement | undefined) {
		if (!hasImage || !pane) return;
		event.preventDefault();
		applyZoomAt(
			pane,
			event.clientX,
			event.clientY,
			clampZoom(zoom * Math.exp(-event.deltaY * 0.0015))
		);
	}

	function applyZoomAt(pane: HTMLElement, clientX: number, clientY: number, nextZoom: number) {
		const size = paneMediaSize();
		if (!size) return;
		const bounds = pane.getBoundingClientRect();
		const cursorX = clientX - bounds.left;
		const cursorY = clientY - bounds.top;
		const currentFrame = fitFrame(pane, size.width, size.height);
		const relativeX = (cursorX - currentFrame.left) / currentFrame.width;
		const relativeY = (cursorY - currentFrame.top) / currentFrame.height;
		zoom = clampZoom(nextZoom);
		const nextFrame = zoomFrameSize(size.width, size.height);
		lockedFrameWidth = nextFrame.width;
		lockedFrameHeight = nextFrame.height;
		viewOriginX = cursorX - relativeX * nextFrame.width;
		viewOriginY = cursorY - relativeY * nextFrame.height;
		panX = 0;
		panY = 0;
	}

	function pointerDistance() {
		const [first, second] = Object.values(pointers);
		if (!first || !second) return 0;
		return Math.hypot(first.x - second.x, first.y - second.y);
	}

	function pointerMidpoint() {
		const [first, second] = Object.values(pointers);
		if (!first || !second) return undefined;
		return { x: (first.x + second.x) / 2, y: (first.y + second.y) / 2 };
	}

	function normalizeCrop(crop: CropRect): CropRect {
		if (!$sourceMeta) {
			return {
				x: Math.max(0, Math.round(crop.x)),
				y: Math.max(0, Math.round(crop.y)),
				width: Math.max(1, Math.round(crop.width)),
				height: Math.max(1, Math.round(crop.height))
			};
		}
		const width = Math.min($sourceMeta.width, Math.max(1, Math.round(crop.width)));
		const height = Math.min($sourceMeta.height, Math.max(1, Math.round(crop.height)));
		return {
			x: Math.min($sourceMeta.width - width, Math.max(0, Math.round(crop.x))),
			y: Math.min($sourceMeta.height - height, Math.max(0, Math.round(crop.y))),
			width,
			height
		};
	}

	function editableCrop() {
		if (activeCrop) return normalizeCrop(activeCrop);
		return fullImageCrop();
	}

	function setCropField(field: keyof CropRect, value: number) {
		cropDraft = normalizeCrop({ ...editableCrop(), [field]: value });
	}

	function findContentCrop(image: ImageData | undefined): CropRect | undefined {
		if (!image) return undefined;
		const threshold = $outputSettings.alphaThreshold;
		let left = image.width;
		let top = image.height;
		let right = -1;
		let bottom = -1;
		let hasTransparentBounds = false;
		for (let y = 0; y < image.height; y++) {
			for (let x = 0; x < image.width; x++) {
				const alpha = image.data[(y * image.width + x) * 4 + 3]!;
				if (alpha <= threshold) {
					hasTransparentBounds = true;
					continue;
				}
				left = Math.min(left, x);
				top = Math.min(top, y);
				right = Math.max(right, x);
				bottom = Math.max(bottom, y);
			}
		}
		if (!hasTransparentBounds || right < left || bottom < top) return undefined;
		if (left === 0 && top === 0 && right === image.width - 1 && bottom === image.height - 1)
			return undefined;
		return { x: left, y: top, width: right - left + 1, height: bottom - top + 1 };
	}

	async function applyCrop() {
		const crop = normalizeCrop(activeCrop!);
		if (crop.width <= 2 || crop.height <= 2) return;
		const anchor = currentViewAnchor();
		const scaleFactor = $outputSettings.scaleFactor ?? 1;
		updateOutputSettings({
			crop,
			width: Math.max(1, Math.round(crop.width * scaleFactor)),
			height: Math.max(1, Math.round(crop.height * scaleFactor)),
			scaleFactor,
			autoSizeOnUpload: false
		});
		cropDraft = undefined;
		cropResize = undefined;
		cropMode = false;
		await tick();
		applyViewAnchor(anchor);
	}

	function clearCrop() {
		cropDraft = cropMode ? fullImageCrop() : undefined;
		updateOutputSettings({ crop: undefined });
	}

	function cancelCropDraft() {
		cropDraft = cropMode ? normalizeCrop($outputSettings.crop ?? fullImageCrop()) : undefined;
	}

	function cropToContent() {
		if (cropToContentBounds) cropDraft = cropToContentBounds;
	}

	function setRevealValue(value: number) {
		revealValue = Math.min(100, Math.max(0, value));
		updatePreviewSettings({ revealValue });
	}

	async function setPreviewMode(value: string) {
		if (value !== 'side-by-side' && value !== 'ab-reveal') return;
		if (value === mode) return;
		const anchor = currentViewAnchor();
		mode = value;
		updatePreviewSettings({ mode });
		await tick();
		applyViewAnchor(anchor);
	}

	async function toggleCropMode() {
		const anchor = currentViewAnchor();
		const nextCropMode = !cropMode;
		if (nextCropMode) cropDraft = normalizeCrop($outputSettings.crop ?? fullImageCrop());
		else cropResize = undefined;
		cropMode = nextCropMode;
		await tick();
		applyViewAnchor(anchor);
	}

	function setRevealFromPointer(event: PointerEvent) {
		if (!revealPane) return;
		const bounds = revealPane.getBoundingClientRect();
		setRevealValue(((event.clientX - bounds.left) / bounds.width) * 100);
	}

	function onRevealPointerDown(event: PointerEvent) {
		if (!hasImage || !revealPane) return;
		event.preventDefault();
		event.stopPropagation();
		revealDrag = event.pointerId;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		setRevealFromPointer(event);
	}

	function onRevealPointerMove(event: PointerEvent) {
		if (revealDrag !== event.pointerId) return;
		event.preventDefault();
		event.stopPropagation();
		setRevealFromPointer(event);
	}

	function onRevealPointerUp(event: PointerEvent) {
		if (revealDrag !== event.pointerId) return;
		event.preventDefault();
		event.stopPropagation();
		revealDrag = undefined;
		(event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
	}

	function onRevealKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowLeft') setRevealValue(revealValue - 2);
		else if (event.key === 'ArrowRight') setRevealValue(revealValue + 2);
		else if (event.key === 'Home') setRevealValue(0);
		else if (event.key === 'End') setRevealValue(100);
		else return;
		event.preventDefault();
	}

	function onDragOver(event: DragEvent) {
		if (!event.dataTransfer?.types.includes('Files')) return;
		event.preventDefault();
		event.dataTransfer.dropEffect = 'copy';
	}

	function onDrop(event: DragEvent) {
		if (!event.dataTransfer?.files.length) return;
		event.preventDefault();
		const file = [...event.dataTransfer.files].find((item) => item.type.startsWith('image/'));
		if (file) onSelectFile?.(file);
	}
</script>

<figure
	class="relative flex w-full flex-col overflow-hidden border border-border bg-muted/40 {minHeightClass}"
	aria-label="Source and processed output comparison preview"
	ondragover={onDragOver}
	ondrop={onDrop}
>
	<div
		class="flex items-center gap-2 border-b border-border bg-background/80 px-2 py-1.5 backdrop-blur"
	>
		<Tabs value={mode} onValueChange={setPreviewMode} class="shrink-0">
			<TabsList>
				<TabsTrigger value="side-by-side">Side-by-side</TabsTrigger>
				<TabsTrigger value="ab-reveal">A/B reveal</TabsTrigger>
			</TabsList>
		</Tabs>

		<div class="ml-auto flex items-center gap-1.5 text-xs text-muted-foreground">
			{#if $processingProgress}
				<Badge variant="secondary" class="tabular-nums">
					{$processingProgress.stage} · {Math.round($processingProgress.progress * 100)}%
				</Badge>
			{/if}
			<Badge variant="outline" class="gap-1"><MagnifyingGlassIcon /> {zoomLabel}</Badge>
			<Badge variant="outline" class="hidden sm:inline-flex">{sizeLabel}</Badge>
			<Badge variant="outline" class="hidden md:inline-flex">{colorLabel}</Badge>
		</div>

		<div class="flex shrink-0 items-center gap-1">
			<Button
				size="icon-sm"
				variant={cropMode ? 'secondary' : 'ghost'}
				aria-label={cropMode ? 'Crop mode active' : 'Crop'}
				disabled={!hasImage}
				onclick={() => void toggleCropMode()}
				aria-pressed={cropMode}
			>
				<CropIcon />
			</Button>
			<Button
				size="sm"
				variant="ghost"
				aria-label="Fit to window"
				disabled={!hasImage}
				onclick={resetView}
			>
				<ArrowsOutIcon />
				<span class="hidden sm:inline">Fit to window</span>
			</Button>
		</div>
	</div>

	<div class="relative flex flex-1 items-stretch">
		{#if !hasImage}
			<Empty class="flex-1">
				<EmptyHeader>
					<EmptyMedia variant="icon">
						<ImageIcon weight="duotone" />
					</EmptyMedia>
					<EmptyTitle>Drop an image to begin</EmptyTitle>
					<EmptyDescription>
						PNG, JPEG, WebP, or GIF. Everything runs locally — your image never leaves your device.
					</EmptyDescription>
				</EmptyHeader>
				<EmptyContent>
					<Button variant="outline" size="sm" onclick={onChooseImage}>
						<UploadIcon />
						Choose file
					</Button>
				</EmptyContent>
			</Empty>
		{:else if cropMode}
			<div
				bind:this={cropPane}
				role="application"
				aria-label="Crop preview. Drag handles to resize the crop, drag elsewhere to pan, and pinch or scroll to zoom."
				class="relative flex-1 cursor-grab touch-none overflow-hidden bg-[repeating-conic-gradient(theme(colors.muted)_0%_25%,transparent_0%_50%)_50%_/_16px_16px] active:cursor-grabbing"
				onpointerdown={(event) => onPointerDown(event, cropPane)}
				onpointermove={(event) => onPointerMove(event, cropPane)}
				onpointerup={onPointerUp}
				onpointercancel={onPointerUp}
				onwheel={(event) => onWheel(event, cropPane)}
			>
				{@render sourceLayer('Source', cropPane)}
			</div>
		{:else if mode === 'side-by-side'}
			<div class="grid flex-1 grid-cols-2 divide-x divide-border">
				<div
					bind:this={sideSourcePane}
					role="application"
					aria-label="Source preview. Drag to pan, scroll to zoom, or enable crop and drag to select a crop."
					class="relative touch-none overflow-hidden bg-[repeating-conic-gradient(theme(colors.muted)_0%_25%,transparent_0%_50%)_50%_/_16px_16px] {cropMode
						? 'cursor-crosshair'
						: 'cursor-grab active:cursor-grabbing'}"
					onpointerdown={(event) => onPointerDown(event, sideSourcePane)}
					onpointermove={(event) => onPointerMove(event, sideSourcePane)}
					onpointerup={onPointerUp}
					onpointercancel={onPointerUp}
					onwheel={(event) => onWheel(event, sideSourcePane)}
				>
					{@render sourceLayer('Source', sideSourcePane)}
				</div>
				<div
					bind:this={sideOutputPane}
					role="application"
					aria-label="Output preview. Drag to pan or scroll to zoom."
					class="relative cursor-grab touch-none overflow-hidden bg-[repeating-conic-gradient(theme(colors.muted)_0%_25%,transparent_0%_50%)_50%_/_16px_16px] active:cursor-grabbing"
					onpointerdown={(event) => onPointerDown(event, sideOutputPane)}
					onpointermove={(event) => onPointerMove(event, sideOutputPane)}
					onpointerup={onPointerUp}
					onpointercancel={onPointerUp}
					onwheel={(event) => onWheel(event, sideOutputPane)}
				>
					{@render outputLayer('Output', sideOutputPane, 'side')}
				</div>
			</div>
		{:else}
			<div
				bind:this={revealPane}
				role="application"
				aria-label="A/B preview. Left side is source, right side is output. Drag the divider to compare, drag the image to pan, scroll to zoom, or enable crop and drag to select a crop."
				class="relative flex-1 touch-none overflow-hidden bg-[repeating-conic-gradient(theme(colors.muted)_0%_25%,transparent_0%_50%)_50%_/_16px_16px] {cropMode
					? 'cursor-crosshair'
					: 'cursor-grab active:cursor-grabbing'}"
				onpointerdown={(event) => onPointerDown(event, revealPane)}
				onpointermove={(event) => onPointerMove(event, revealPane)}
				onpointerup={onPointerUp}
				onpointercancel={onPointerUp}
				onwheel={(event) => onWheel(event, revealPane)}
			>
				{@render outputLayer('Output', revealPane, 'reveal')}
				<div class="absolute inset-y-0 left-0 overflow-hidden" style="width: {revealValue}%">
					{@render sourceLayer('Source', revealPane)}
				</div>
				<div
					role="slider"
					tabindex="0"
					aria-label="A/B reveal divider"
					aria-valuemin="0"
					aria-valuemax="100"
					aria-valuenow={Math.round(revealValue)}
					class="absolute top-0 bottom-0 z-20 flex w-8 -translate-x-1/2 cursor-ew-resize items-center justify-center focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
					style="left: {revealValue}%"
					onpointerdown={onRevealPointerDown}
					onpointermove={onRevealPointerMove}
					onpointerup={onRevealPointerUp}
					onpointercancel={onRevealPointerUp}
					onkeydown={onRevealKeydown}
				>
					<span class="h-full w-px bg-foreground/80" aria-hidden="true"></span>
					<span
						class="absolute top-1/2 grid size-8 -translate-y-1/2 place-items-center border border-foreground bg-background text-muted-foreground shadow-sm"
						aria-hidden="true"
					>
						↔
					</span>
				</div>
			</div>
		{/if}
	</div>

	{#if cropMode && hasImage}
		<figcaption
			class="grid gap-2 border-t border-border bg-background/90 px-3 py-2 text-xs text-muted-foreground"
		>
			<span
				>Drag crop handles or edit exact crop fields. Drag elsewhere to pan. Output is hidden while
				cropping.</span
			>
			<div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
				<div class="grid gap-1">
					<Label for="crop-x" class="text-xs">X</Label>
					<Input
						id="crop-x"
						type="number"
						inputmode="numeric"
						min="0"
						max={$sourceMeta ? $sourceMeta.width - 1 : undefined}
						step="1"
						value={editableCrop().x}
						oninput={(event) => setCropField('x', Number(event.currentTarget.value))}
					/>
				</div>
				<div class="grid gap-1">
					<Label for="crop-y" class="text-xs">Y</Label>
					<Input
						id="crop-y"
						type="number"
						inputmode="numeric"
						min="0"
						max={$sourceMeta ? $sourceMeta.height - 1 : undefined}
						step="1"
						value={editableCrop().y}
						oninput={(event) => setCropField('y', Number(event.currentTarget.value))}
					/>
				</div>
				<div class="grid gap-1">
					<Label for="crop-width" class="text-xs">Width</Label>
					<Input
						id="crop-width"
						type="number"
						inputmode="numeric"
						min="1"
						max={$sourceMeta ? $sourceMeta.width : undefined}
						step="1"
						value={editableCrop().width}
						oninput={(event) => setCropField('width', Number(event.currentTarget.value))}
					/>
				</div>
				<div class="grid gap-1">
					<Label for="crop-height" class="text-xs">Height</Label>
					<Input
						id="crop-height"
						type="number"
						inputmode="numeric"
						min="1"
						max={$sourceMeta ? $sourceMeta.height : undefined}
						step="1"
						value={editableCrop().height}
						oninput={(event) => setCropField('height', Number(event.currentTarget.value))}
					/>
				</div>
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<Button size="xs" variant="outline" onclick={applyCrop} disabled={!activeCrop}
					>Apply crop</Button
				>
				<Button
					size="xs"
					variant="ghost"
					onclick={cropToContent}
					disabled={!canCropToContent}
					title={cropToContentHint}>Crop to content</Button
				>
				<Button size="xs" variant="ghost" onclick={clearCrop} disabled={!activeCrop}
					>Reset crop</Button
				>
				<Button size="xs" variant="ghost" onclick={cancelCropDraft} disabled={!cropDraft}
					>Cancel draft</Button
				>
				<span>{cropToContentHint}</span>
			</div>
		</figcaption>
	{:else if $processingError}
		<figcaption
			class="border-t border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
		>
			{$processingError}
		</figcaption>
	{:else if $processedImage?.warnings.length}
		<figcaption
			class="border-t border-border bg-background/90 px-3 py-2 text-xs text-muted-foreground"
		>
			{$processedImage.warnings[0]}
		</figcaption>
	{/if}
</figure>

{#snippet sourceLayer(label: string, pane: HTMLElement | undefined)}
	<span class="absolute top-2 left-2 z-10 bg-background/80 px-2 py-0.5 text-xs font-medium"
		>{label}</span
	>
	{#if $sourceObjectUrl && $sourceMeta}
		<img
			src={$sourceObjectUrl}
			alt="Uploaded source"
			class="pointer-events-none absolute max-w-none select-none [image-rendering:auto]"
			style={mediaStyle(pane, $sourceMeta.width, $sourceMeta.height)}
			draggable="false"
		/>
		{#if activeCrop}
			<div
				class="pointer-events-none absolute border-2 border-primary/80 shadow-[0_0_0_9999px_rgba(0,0,0,0.35)]"
				style={cropStyle(pane, activeCrop)}
			>
				{#if cropMode}
					{#each CROP_HANDLES as handle (handle)}
						<button
							type="button"
							class={cropHandleClass(handle)}
							aria-label={`Resize crop ${handle}`}
							onpointerdown={(event) => onCropHandlePointerDown(event, handle, pane)}
						></button>
					{/each}
				{/if}
			</div>
		{/if}
	{/if}
{/snippet}

{#snippet outputLayer(label: string, pane: HTMLElement | undefined, target: 'side' | 'reveal')}
	<span class="absolute top-2 left-2 z-10 bg-background/80 px-2 py-0.5 text-xs font-medium"
		>{label}</span
	>
	{#if $processedImage}
		{@const style = outputMediaStyle(pane, $processedImage.width, $processedImage.height)}
		{#if target === 'side'}
			<canvas
				bind:this={sideOutputCanvas}
				class="pointer-events-none absolute max-w-none select-none [image-rendering:auto]"
				{style}
				aria-label="Processed dithered output"
			></canvas>
		{:else}
			<canvas
				bind:this={revealOutputCanvas}
				class="pointer-events-none absolute max-w-none select-none [image-rendering:auto]"
				{style}
				aria-label="Processed dithered output"
			></canvas>
		{/if}
	{:else}
		<div class="flex h-full w-full items-center justify-center text-sm text-muted-foreground">
			Processing…
		</div>
	{/if}
{/snippet}
