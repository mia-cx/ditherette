<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Input } from '$lib/components/ui/input';
	import { Slider } from '$lib/components/ui/slider';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { RESIZE_MODES } from './output-options';
	import { outputSettings, sourceMeta, updateOutputSettings } from '$lib/stores/app';
	import { clampOutputScale, fitOutputSizeToBounds } from '$lib/processing/types';

	type Props = {
		hasImage?: boolean;
		hideHeading?: boolean;
	};

	let { hasImage = false, hideHeading = false }: Props = $props();

	const SLIDER_MIN_SCALE = 0.05;
	const SLIDER_MAX_SCALE = 1;
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
	const maximumScale = $derived(clampScale(Number.MAX_VALUE));
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
		scaleFactor = clampScale(scaleFactor);
		const dimensions = dimensionsForScale(scaleFactor);
		if (width !== dimensions.width || height !== dimensions.height) {
			width = dimensions.width;
			height = dimensions.height;
		}
	});

	$effect(() => {
		const clamped = fitOutputSizeToBounds(width, height);
		updateOutputSettings({
			width: clamped.width,
			height: clamped.height,
			lockAspect: true,
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

	function dimensionsForAspect(nextWidth: number, aspect: number) {
		const safeWidth = Math.max(1, Math.round(nextWidth || 1));
		return { width: safeWidth, height: Math.max(1, Math.round(safeWidth / aspect)) };
	}

	function clampScale(value: number) {
		const base = baseDimensions ?? { width, height };
		return clampOutputScale(value, base.width, base.height);
	}

	function dimensionsForScale(value: number) {
		const nextScale = clampScale(value);
		const base = baseDimensions ?? { width, height };
		const nextWidth = Math.max(1, Math.round(base.width * nextScale));
		const nextHeight = Math.max(1, Math.round(base.height * nextScale));
		return fitOutputSizeToBounds(nextWidth, nextHeight);
	}

	function setScale(value: number) {
		if (!Number.isFinite(value) || value <= 0) return;
		scaleFactor = clampScale(value);
		const dimensions = dimensionsForScale(scaleFactor);
		width = dimensions.width;
		height = dimensions.height;
	}

	function setWidth(value: number) {
		const dimensions = dimensionsForAspect(value, enforcedAspectRatio);
		setScale(factorFromDimensions(dimensions.width, dimensions.height));
	}

	function setHeight(value: number) {
		const safeHeight = Math.max(1, Math.round(value || 1));
		const nextWidth = Math.max(1, Math.round(safeHeight * enforcedAspectRatio));
		setScale(factorFromDimensions(nextWidth, safeHeight));
	}

	function commitOnEnter(event: KeyboardEvent) {
		if (event.key !== 'Enter') return;
		(event.currentTarget as HTMLInputElement).blur();
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
		<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
			<Label for="scale-ratio" class="text-xs text-muted-foreground">Scale</Label>
			<Slider
				type="single"
				value={Math.max(SLIDER_MIN_SCALE, Math.min(SLIDER_MAX_SCALE, scaleFactor))}
				min={SLIDER_MIN_SCALE}
				max={SLIDER_MAX_SCALE}
				step={SCALE_STEP}
				disabled={!hasImage}
				aria-label="Output scale factor"
				onValueChange={setScale}
			/>
			<div class="relative">
				<Input
					id="scale-ratio"
					class="h-8 bg-background pr-5 text-right font-mono text-xs tabular-nums"
					type="number"
					inputmode="decimal"
					min={Number.MIN_VALUE}
					max={maximumScale}
					step="any"
					value={scaleFactor}
					disabled={!hasImage}
					onkeydown={commitOnEnter}
					onchange={(event) => {
						const input = event.currentTarget as HTMLInputElement;
						setScale(input.valueAsNumber);
						input.value = String(scaleFactor);
					}}
				/>
				<span
					class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
					>×</span
				>
			</div>
		</div>

		<div class="grid gap-2">
			<div class="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-2">
				<Label for="out-width" class="text-xs text-muted-foreground">Width</Label>
				<div class="relative">
					<Input
						id="out-width"
						class="bg-background pr-7 text-right font-mono tabular-nums"
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
					<span
						class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
						>px</span
					>
				</div>
			</div>
			<div class="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-2">
				<Label for="out-height" class="text-xs text-muted-foreground">Height</Label>
				<div class="relative">
					<Input
						id="out-height"
						class="bg-background pr-7 text-right font-mono tabular-nums"
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
					<span
						class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
						>px</span
					>
				</div>
			</div>
			<div class="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-2">
				<Label for="resize-mode" class="text-xs text-muted-foreground">Resample</Label>
				<Select bind:value={resize} type="single">
					<SelectTrigger id="resize-mode" class="w-full">{resizeLabel}</SelectTrigger>
					<SelectContent>
						{#each RESIZE_MODES as r (r.id)}
							<SelectItem value={r.id}>{r.label}</SelectItem>
						{/each}
					</SelectContent>
				</Select>
			</div>
		</div>
	</div>
</section>
