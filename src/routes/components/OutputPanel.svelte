<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import { Slider } from '$lib/components/ui/slider';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { RESIZE_MODES } from './sample-data';
	import { outputSettings, sourceMeta, updateOutputSettings } from '$lib/stores/app';
	import { fitOutputSizeToBounds } from '$lib/processing/types';

	type Props = {
		hasImage?: boolean;
		hideHeading?: boolean;
	};

	let { hasImage = false, hideHeading = false }: Props = $props();

	const MIN_SCALE = 0.05;
	const MAX_SCALE = 4;
	const SCALE_STEP = 0.01;
	const initial = outputSettings.get();
	let width = $state<number>(initial.width);
	let height = $state<number>(initial.height);
	let resize = $state(initial.resize);
	let scaleValue = $state(1);
	let lastEditedDimension = $state<'width' | 'height'>('width');

	const resizeLabel = $derived(RESIZE_MODES.find((r) => r.id === resize)?.label ?? 'Resize');
	const baseDimensions = $derived.by(() => {
		const crop = $outputSettings.crop;
		if (crop) return { width: Math.max(1, crop.width), height: Math.max(1, crop.height) };
		if ($sourceMeta) return { width: $sourceMeta.width, height: $sourceMeta.height };
		return { width, height };
	});
	const enforcedAspectRatio = $derived(validRatio(baseDimensions.width, baseDimensions.height));
	const scaleRatio = $derived.by(() => {
		if (lastEditedDimension === 'height') return height / baseDimensions.height;
		return width / baseDimensions.width;
	});
	const scaleDisplay = $derived(formatScale(scaleRatio));

	$effect(() =>
		outputSettings.subscribe((settings) => {
			width = settings.width;
			height = settings.height;
			resize = settings.resize;
			scaleValue = Math.max(MIN_SCALE, Math.min(MAX_SCALE, scaleRatio));
		})
	);

	$effect(() => {
		const dimensions = dimensionsForAspect(width, height, enforcedAspectRatio);
		const clamped = fitOutputSizeToBounds(dimensions.width, dimensions.height);
		updateOutputSettings({
			width: clamped.width,
			height: clamped.height,
			lockAspect: true,
			fit: 'stretch',
			resize,
			autoSizeOnUpload: false
		});
	});

	function validRatio(nextWidth: number, nextHeight: number) {
		return nextHeight > 0 ? Math.max(1 / 16_384, nextWidth / nextHeight) : 1;
	}

	function formatScale(value: number) {
		return `${Number.isFinite(value) ? value.toFixed(2) : '1.00'}×`;
	}

	function dimensionsForAspect(nextWidth: number, nextHeight: number, aspect: number) {
		if (lastEditedDimension === 'height') {
			const safeHeight = Math.max(1, Math.round(nextHeight || 1));
			return { width: Math.max(1, Math.round(safeHeight * aspect)), height: safeHeight };
		}
		const safeWidth = Math.max(1, Math.round(nextWidth || 1));
		return { width: safeWidth, height: Math.max(1, Math.round(safeWidth / aspect)) };
	}

	function dimensionsForScale(value: number) {
		const nextScale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, value || 1));
		const nextWidth = Math.max(1, Math.round(baseDimensions.width * nextScale));
		const nextHeight = Math.max(1, Math.round(baseDimensions.height * nextScale));
		return fitOutputSizeToBounds(nextWidth, nextHeight);
	}

	function setScale(value: number) {
		scaleValue = value;
		const dimensions = dimensionsForScale(value);
		width = dimensions.width;
		height = dimensions.height;
	}

	function setWidth(value: number) {
		lastEditedDimension = 'width';
		const dimensions = dimensionsForAspect(value, height, enforcedAspectRatio);
		width = dimensions.width;
		height = dimensions.height;
	}

	function setHeight(value: number) {
		lastEditedDimension = 'height';
		const dimensions = dimensionsForAspect(width, value, enforcedAspectRatio);
		width = dimensions.width;
		height = dimensions.height;
	}

	function resetDimensionsToCrop() {
		const crop = $outputSettings.crop;
		if (!crop) return;
		lastEditedDimension = 'width';
		width = Math.max(1, Math.round(crop.width));
		height = Math.max(1, Math.round(crop.height));
		updateOutputSettings({
			width,
			height,
			lockAspect: true,
			fit: 'stretch',
			autoSizeOnUpload: false
		});
	}

	function clearCrop() {
		updateOutputSettings({ crop: undefined });
	}
</script>

<section class="flex flex-col gap-3" aria-label="Dimension controls">
	{#if !hideHeading}
		<div class="flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold tracking-tight">Dimensions</h2>
			<p class="text-xs text-muted-foreground">Output size and resampling.</p>
		</div>
	{/if}

	<div class="grid max-w-sm gap-3 text-sm">
		<div class="grid gap-1.5">
			<div class="flex items-center justify-between gap-3">
				<Label for="scale-ratio">Scale</Label>
				<span class="font-mono text-xs text-muted-foreground tabular-nums">{scaleDisplay}</span>
			</div>
			<Slider
				type="single"
				bind:value={scaleValue}
				min={MIN_SCALE}
				max={MAX_SCALE}
				step={SCALE_STEP}
				disabled={!hasImage}
				aria-label="Output scale ratio"
				onValueChange={setScale}
			/>
		</div>

		<div class="grid grid-cols-2 gap-2">
			<div class="grid gap-1">
				<Input
					id="out-width"
					type="number"
					inputmode="numeric"
					min="1"
					max="16384"
					step="1"
					value={width}
					disabled={!hasImage}
					oninput={(event) => setWidth(Number((event.currentTarget as HTMLInputElement).value))}
				/>
				<Label for="out-width" class="text-xs text-muted-foreground">Width</Label>
			</div>
			<div class="grid gap-1">
				<Input
					id="out-height"
					type="number"
					inputmode="numeric"
					min="1"
					max="16384"
					step="1"
					value={height}
					disabled={!hasImage}
					oninput={(event) => setHeight(Number((event.currentTarget as HTMLInputElement).value))}
				/>
				<Label for="out-height" class="text-xs text-muted-foreground">Height</Label>
			</div>
		</div>

		<div class="grid grid-cols-[6rem_minmax(8rem,12rem)] items-center justify-start gap-2">
			<Label for="resize-mode">Resample</Label>
			<Select bind:value={resize} type="single">
				<SelectTrigger id="resize-mode">{resizeLabel}</SelectTrigger>
				<SelectContent>
					{#each RESIZE_MODES as r (r.id)}
						<SelectItem value={r.id}>{r.label}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
		</div>
	</div>

	{#if $outputSettings.crop}
		<div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
			<span class="font-mono">
				Crop {$outputSettings.crop.width.toFixed(0)}×{$outputSettings.crop.height.toFixed(0)}
			</span>
			<Button size="xs" variant="outline" onclick={resetDimensionsToCrop}
				>Reset dimensions to crop</Button
			>
			<Button size="xs" variant="ghost" onclick={clearCrop}>Clear crop</Button>
		</div>
	{/if}
</section>
