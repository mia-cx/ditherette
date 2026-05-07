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
	const MAX_SCALE = 1;
	const SCALE_STEP = 0.0001;
	const initial = outputSettings.get();
	let width = $state<number>(initial.width);
	let height = $state<number>(initial.height);
	let resize = $state(initial.resize);
	let scaleFactor = $state<number>(initial.scaleFactor ?? 1);

	const resizeLabel = $derived(RESIZE_MODES.find((r) => r.id === resize)?.label ?? 'Resize');
	const baseDimensions = $derived.by(() => {
		const crop = $outputSettings.crop;
		if (crop) return { width: Math.max(1, crop.width), height: Math.max(1, crop.height) };
		if ($sourceMeta) return { width: $sourceMeta.width, height: $sourceMeta.height };
		return undefined;
	});
	const enforcedAspectRatio = $derived(
		baseDimensions
			? validRatio(baseDimensions.width, baseDimensions.height)
			: validRatio(width, height)
	);
	$effect(() =>
		outputSettings.subscribe((settings) => {
			width = settings.width;
			height = settings.height;
			resize = settings.resize;
			scaleFactor = clampScale(
				settings.scaleFactor ?? factorFromDimensions(settings.width, settings.height)
			);
		})
	);

	$effect(() => {
		if (!baseDimensions) return;
		const dimensions = dimensionsForScale(scaleFactor);
		if (width !== dimensions.width || height !== dimensions.height) {
			width = dimensions.width;
			height = dimensions.height;
		}
	});

	$effect(() => {
		const dimensions = dimensionsForAspect(width, height, enforcedAspectRatio);
		const clamped = fitOutputSizeToBounds(dimensions.width, dimensions.height);
		updateOutputSettings({
			width: clamped.width,
			height: clamped.height,
			lockAspect: true,
			fit: 'stretch',
			resize,
			autoSizeOnUpload: false,
			scaleFactor
		});
	});

	function validRatio(nextWidth: number, nextHeight: number) {
		return nextHeight > 0 ? Math.max(1 / 16_384, nextWidth / nextHeight) : 1;
	}

	function factorFromDimensions(nextWidth: number, nextHeight: number) {
		if (!baseDimensions) return scaleFactor;
		const nextFactor =
			validRatio(baseDimensions.width, baseDimensions.height) >= 1
				? nextWidth / baseDimensions.width
				: nextHeight / baseDimensions.height;
		return clampScale(nextFactor);
	}

	function dimensionsForAspect(nextWidth: number, nextHeight: number, aspect: number) {
		const safeWidth = Math.max(1, Math.round(nextWidth || 1));
		return { width: safeWidth, height: Math.max(1, Math.round(safeWidth / aspect)) };
	}

	function clampScale(value: number) {
		return Math.max(MIN_SCALE, Math.min(MAX_SCALE, value || 1));
	}

	function dimensionsForScale(value: number) {
		const nextScale = clampScale(value);
		const base = baseDimensions ?? { width, height };
		const nextWidth = Math.max(1, Math.round(base.width * nextScale));
		const nextHeight = Math.max(1, Math.round(base.height * nextScale));
		return fitOutputSizeToBounds(nextWidth, nextHeight);
	}

	function setScale(value: number) {
		scaleFactor = clampScale(value);
		const dimensions = dimensionsForScale(scaleFactor);
		width = dimensions.width;
		height = dimensions.height;
	}

	function setWidth(value: number) {
		const dimensions = dimensionsForAspect(value, height, enforcedAspectRatio);
		setScale(factorFromDimensions(dimensions.width, dimensions.height));
	}

	function setHeight(value: number) {
		const safeHeight = Math.max(1, Math.round(value || 1));
		const nextWidth = Math.max(1, Math.round(safeHeight * enforcedAspectRatio));
		setScale(factorFromDimensions(nextWidth, safeHeight));
	}

	function resetDimensionsToCrop() {
		const crop = $outputSettings.crop;
		if (!crop) return;
		const dimensions = dimensionsForScale(scaleFactor);
		width = dimensions.width;
		height = dimensions.height;
		updateOutputSettings({
			width,
			height,
			lockAspect: true,
			fit: 'stretch',
			scaleFactor,
			autoSizeOnUpload: false
		});
	}

	function commitOnEnter(event: KeyboardEvent) {
		if (event.key !== 'Enter') return;
		(event.currentTarget as HTMLInputElement).blur();
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

	<div class="grid w-full gap-3 text-sm">
		<div class="grid gap-1.5">
			<Label for="scale-ratio">Scale</Label>
			<div class="grid grid-cols-[minmax(0,1fr)_6rem] items-center gap-3">
				<Slider
					type="single"
					bind:value={scaleFactor}
					min={MIN_SCALE}
					max={MAX_SCALE}
					step={SCALE_STEP}
					disabled={!hasImage}
					aria-label="Output scale factor"
					onValueChange={setScale}
				/>
				<div class="relative">
					<Input
						id="scale-ratio"
						class="h-8 pr-5 text-right font-mono tabular-nums"
						type="number"
						inputmode="decimal"
						min={MIN_SCALE}
						max={MAX_SCALE}
						step={SCALE_STEP}
						value={Number.isFinite(scaleFactor) ? Number(scaleFactor.toFixed(4)) : 1}
						disabled={!hasImage}
						onkeydown={commitOnEnter}
						onchange={(event) => setScale(Number((event.currentTarget as HTMLInputElement).value))}
					/>
					<span
						class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
						>×</span
					>
				</div>
			</div>
		</div>

		<div class="grid grid-cols-2 gap-2">
			<div class="grid gap-1">
				<Input
					id="out-width"
					class="text-right font-mono tabular-nums"
					type="number"
					inputmode="numeric"
					min="1"
					max="16384"
					step="1"
					value={width}
					disabled={!hasImage}
					onkeydown={commitOnEnter}
					onchange={(event) => setWidth(Number((event.currentTarget as HTMLInputElement).value))}
				/>
				<Label for="out-width" class="text-xs text-muted-foreground">Width</Label>
			</div>
			<div class="grid gap-1">
				<Input
					id="out-height"
					class="text-right font-mono tabular-nums"
					type="number"
					inputmode="numeric"
					min="1"
					max="16384"
					step="1"
					value={height}
					disabled={!hasImage}
					onkeydown={commitOnEnter}
					onchange={(event) => setHeight(Number((event.currentTarget as HTMLInputElement).value))}
				/>
				<Label for="out-height" class="text-xs text-muted-foreground">Height</Label>
			</div>
		</div>

		<div class="grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2">
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
			<Button size="xs" variant="outline" onclick={resetDimensionsToCrop}
				>Reset dimensions to crop scale</Button
			>
			<Button size="xs" variant="ghost" onclick={clearCrop}>Clear crop</Button>
		</div>
	{/if}
</section>
