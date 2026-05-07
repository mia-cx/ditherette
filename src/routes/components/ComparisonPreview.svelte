<script lang="ts">
	import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import {
		Empty,
		EmptyContent,
		EmptyDescription,
		EmptyHeader,
		EmptyMedia,
		EmptyTitle
	} from '$lib/components/ui/empty';
	import { drawImageDataToCanvas, processedToImageData } from '$lib/processing/render';
	import type { CropRect } from '$lib/processing/types';
	import {
		outputSettings,
		previewSettings,
		processedImage,
		processingError,
		processingProgress,
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

	type Props = {
		defaultMode?: PreviewMode;
		hasImage?: boolean;
		minHeightClass?: string;
		onChooseImage?: () => void;
	};

	let {
		defaultMode = 'side-by-side',
		hasImage = false,
		minHeightClass = 'min-h-[320px] md:min-h-[420px]',
		onChooseImage
	}: Props = $props();

	// svelte-ignore state_referenced_locally -- defaultMode intentionally seeds user-controlled local state.
	let mode = $state<PreviewMode>(previewSettings.get().mode ?? defaultMode);
	let revealValue = $state<number>(previewSettings.get().revealValue ?? 50);
	let revealDrag = $state<number>();
	let zoom = $state(1);
	let panX = $state(0);
	let panY = $state(0);
	let cropMode = $state(false);
	let cropDraft = $state<CropRect>();
	let cropStart = $state<Point>();
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
	const activeCrop = $derived(cropDraft ?? $outputSettings.crop);

	$effect(() => {
		if (!$processedImage) return;
		const imageData = processedToImageData($processedImage);
		if (sideOutputCanvas) drawImageDataToCanvas(sideOutputCanvas, imageData);
		if (revealOutputCanvas) drawImageDataToCanvas(revealOutputCanvas, imageData);
	});

	$effect(() =>
		previewSettings.subscribe((settings) => {
			mode = settings.mode ?? defaultMode;
			revealValue = settings.revealValue ?? 50;
		})
	);

	function resetView() {
		zoom = 1;
		panX = 0;
		panY = 0;
	}

	function fitFrame(pane: HTMLElement | undefined, width: number, height: number) {
		if (!pane) return { left: 0, top: 0, width: 0, height: 0 };
		const paneWidth = pane.clientWidth;
		const paneHeight = pane.clientHeight;
		const scale = Math.min(paneWidth / width, paneHeight / height);
		const baseWidth = width * scale;
		const baseHeight = height * scale;
		const scaledWidth = baseWidth * zoom;
		const scaledHeight = baseHeight * zoom;
		return {
			left: (paneWidth - baseWidth) / 2 + panX - (scaledWidth - baseWidth) / 2,
			top: (paneHeight - baseHeight) / 2 + panY - (scaledHeight - baseHeight) / 2,
			width: scaledWidth,
			height: scaledHeight
		};
	}

	function mediaStyle(
		pane: HTMLElement | undefined,
		width: number | undefined,
		height: number | undefined
	) {
		if (!width || !height) return '';
		const frame = fitFrame(pane, width, height);
		return `left:${frame.left}px;top:${frame.top}px;width:${frame.width}px;height:${frame.height}px`;
	}

	function cropStyle(pane: HTMLElement | undefined, crop: CropRect | undefined) {
		if (!pane || !crop || !$sourceMeta) return '';
		const frame = fitFrame(pane, $sourceMeta.width, $sourceMeta.height);
		return [
			`left:${frame.left + (crop.x / $sourceMeta.width) * frame.width}px`,
			`top:${frame.top + (crop.y / $sourceMeta.height) * frame.height}px`,
			`width:${(crop.width / $sourceMeta.width) * frame.width}px`,
			`height:${(crop.height / $sourceMeta.height) * frame.height}px`
		].join(';');
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

	function cropFromPoints(start: Point, end: Point): CropRect {
		const x = Math.min(start.x, end.x);
		const y = Math.min(start.y, end.y);
		return {
			x,
			y,
			width: Math.max(1, Math.abs(end.x - start.x)),
			height: Math.max(1, Math.abs(end.y - start.y))
		};
	}

	function onPointerDown(event: PointerEvent, pane: HTMLElement | undefined, allowCrop: boolean) {
		if (!hasImage || !pane || event.button !== 0) return;
		event.preventDefault();
		pane.setPointerCapture(event.pointerId);

		if (cropMode && allowCrop) {
			const start = panePointToImage(pane, event.clientX, event.clientY);
			if (!start) return;
			cropStart = start;
			cropDraft = { x: start.x, y: start.y, width: 1, height: 1 };
			return;
		}

		drag = { pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, panX, panY };
	}

	function onPointerMove(event: PointerEvent, pane: HTMLElement | undefined) {
		if (cropStart && pane) {
			const end = panePointToImage(pane, event.clientX, event.clientY);
			if (end) cropDraft = cropFromPoints(cropStart, end);
			return;
		}
		if (!drag || drag.pointerId !== event.pointerId) return;
		panX = drag.panX + event.clientX - drag.startX;
		panY = drag.panY + event.clientY - drag.startY;
	}

	function onPointerUp(event: PointerEvent) {
		if (cropStart && cropDraft) {
			const crop = {
				x: Math.round(cropDraft.x),
				y: Math.round(cropDraft.y),
				width: Math.max(1, Math.round(cropDraft.width)),
				height: Math.max(1, Math.round(cropDraft.height))
			};
			if (crop.width > 2 && crop.height > 2) {
				updateOutputSettings({ crop, width: crop.width, height: crop.height });
			}
		}
		cropStart = undefined;
		cropDraft = undefined;
		drag = undefined;
		if (event.currentTarget instanceof HTMLElement)
			event.currentTarget.releasePointerCapture(event.pointerId);
	}

	function onWheel(event: WheelEvent, pane: HTMLElement | undefined) {
		if (!hasImage || !pane) return;
		event.preventDefault();
		const bounds = pane.getBoundingClientRect();
		const nextZoom = Math.min(16, Math.max(0.25, zoom * Math.exp(-event.deltaY * 0.0015)));
		const cursorX = event.clientX - bounds.left - bounds.width / 2;
		const cursorY = event.clientY - bounds.top - bounds.height / 2;
		const ratio = nextZoom / zoom;
		panX = cursorX - ratio * (cursorX - panX);
		panY = cursorY - ratio * (cursorY - panY);
		zoom = nextZoom;
	}

	function setRevealValue(value: number) {
		revealValue = Math.min(100, Math.max(0, value));
		updatePreviewSettings({ revealValue });
	}

	function setPreviewMode(value: string) {
		if (value !== 'side-by-side' && value !== 'ab-reveal') return;
		mode = value;
		updatePreviewSettings({ mode });
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
</script>

<figure
	class="relative flex w-full flex-col overflow-hidden border border-border bg-muted/40 {minHeightClass}"
	aria-label="Source and processed output comparison preview"
>
	<div
		class="flex items-center gap-2 border-b border-border bg-background/80 px-2 py-1.5 backdrop-blur"
	>
		<Tabs bind:value={mode} onValueChange={setPreviewMode} class="shrink-0">
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
				onclick={() => (cropMode = !cropMode)}
				aria-pressed={cropMode}
			>
				<CropIcon />
			</Button>
			<Button
				size="icon-sm"
				variant="ghost"
				aria-label="Fit to view"
				disabled={!hasImage}
				onclick={resetView}
			>
				<ArrowsOutIcon />
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
		{:else if mode === 'side-by-side'}
			<div class="grid flex-1 grid-cols-2 divide-x divide-border">
				<div
					bind:this={sideSourcePane}
					role="application"
					aria-label="Source preview. Drag to pan, scroll to zoom, or enable crop and drag to select a crop."
					class="relative touch-none overflow-hidden bg-[repeating-conic-gradient(theme(colors.muted)_0%_25%,transparent_0%_50%)_50%_/_16px_16px] {cropMode
						? 'cursor-crosshair'
						: 'cursor-grab active:cursor-grabbing'}"
					onpointerdown={(event) => onPointerDown(event, sideSourcePane, true)}
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
					onpointerdown={(event) => onPointerDown(event, sideOutputPane, false)}
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
				onpointerdown={(event) => onPointerDown(event, revealPane, true)}
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
			class="border-t border-border bg-background/90 px-3 py-2 text-xs text-muted-foreground"
		>
			Drag on the source image to crop. Output dimensions reset to the crop size when you release.
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
				class="pointer-events-none absolute border-2 border-primary bg-primary/15 shadow-[0_0_0_9999px_rgba(0,0,0,0.35)]"
				style={cropStyle(pane, activeCrop)}
			></div>
		{/if}
	{/if}
{/snippet}

{#snippet outputLayer(label: string, pane: HTMLElement | undefined, target: 'side' | 'reveal')}
	<span class="absolute top-2 left-2 z-10 bg-background/80 px-2 py-0.5 text-xs font-medium"
		>{label}</span
	>
	{#if $processedImage}
		{@const style = mediaStyle(
			pane,
			$sourceMeta?.width ?? $processedImage.width,
			$sourceMeta?.height ?? $processedImage.height
		)}
		{#if target === 'side'}
			<canvas
				bind:this={sideOutputCanvas}
				class="pointer-events-none absolute max-w-none select-none [image-rendering:pixelated]"
				{style}
				aria-label="Processed dithered output"
			></canvas>
		{:else}
			<canvas
				bind:this={revealOutputCanvas}
				class="pointer-events-none absolute max-w-none select-none [image-rendering:pixelated]"
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
