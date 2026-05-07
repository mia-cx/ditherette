<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { RESIZE_MODES } from './sample-data';
	import { outputSettings, sourceMeta, updateOutputSettings } from '$lib/stores/app';
	import { fitOutputSizeToBounds } from '$lib/processing/types';

	type Props = {
		hasImage?: boolean;
		hideHeading?: boolean;
	};

	let { hasImage = false, hideHeading = false }: Props = $props();

	const initial = outputSettings.get();
	let width = $state<number>(initial.width);
	let height = $state<number>(initial.height);
	let resize = $state(initial.resize);
	let autoSizeOnUpload = $state(initial.autoSizeOnUpload ?? true);
	let lastEditedDimension = $state<'width' | 'height'>('width');

	const resizeLabel = $derived(RESIZE_MODES.find((r) => r.id === resize)?.label ?? 'Resize');
	const enforcedAspectRatio = $derived.by(() => {
		const crop = $outputSettings.crop;
		if (crop) return validRatio(crop.width, crop.height);
		if ($sourceMeta) return validRatio($sourceMeta.width, $sourceMeta.height);
		return validRatio(width, height);
	});

	$effect(() =>
		outputSettings.subscribe((settings) => {
			width = settings.width;
			height = settings.height;
			resize = settings.resize;
			autoSizeOnUpload = settings.autoSizeOnUpload ?? true;
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
			autoSizeOnUpload
		});
	});

	function validRatio(nextWidth: number, nextHeight: number) {
		return nextHeight > 0 ? Math.max(1 / 16_384, nextWidth / nextHeight) : 1;
	}

	function dimensionsForAspect(nextWidth: number, nextHeight: number, aspect: number) {
		if (lastEditedDimension === 'height') {
			const safeHeight = Math.max(1, Math.round(nextHeight || 1));
			return { width: Math.max(1, Math.round(safeHeight * aspect)), height: safeHeight };
		}
		const safeWidth = Math.max(1, Math.round(nextWidth || 1));
		return { width: safeWidth, height: Math.max(1, Math.round(safeWidth / aspect)) };
	}

	function setWidth(value: number) {
		autoSizeOnUpload = false;
		lastEditedDimension = 'width';
		const dimensions = dimensionsForAspect(value, height, enforcedAspectRatio);
		width = dimensions.width;
		height = dimensions.height;
	}

	function setHeight(value: number) {
		autoSizeOnUpload = false;
		lastEditedDimension = 'height';
		const dimensions = dimensionsForAspect(width, value, enforcedAspectRatio);
		width = dimensions.width;
		height = dimensions.height;
	}

	function resetAspectToSourceOrCrop() {
		autoSizeOnUpload = false;
		const dimensions = dimensionsForAspect(width, height, enforcedAspectRatio);
		width = dimensions.width;
		height = dimensions.height;
		updateOutputSettings({ width, height, lockAspect: true, fit: 'stretch', autoSizeOnUpload });
	}

	function resetDimensionsToCrop() {
		const crop = $outputSettings.crop;
		if (!crop) return;
		autoSizeOnUpload = false;
		width = Math.max(1, Math.round(crop.width));
		height = Math.max(1, Math.round(crop.height));
		lastEditedDimension = 'width';
		updateOutputSettings({ width, height, lockAspect: true, fit: 'stretch', autoSizeOnUpload });
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

	<div class="grid gap-2 text-sm">
		<div class="grid grid-cols-[5.5rem_minmax(0,1fr)] items-center gap-2">
			<Label for="out-width">Width</Label>
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
		</div>
		<div class="grid grid-cols-[5.5rem_minmax(0,1fr)] items-center gap-2">
			<Label for="out-height">Height</Label>
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
		</div>
		<div class="grid grid-cols-[5.5rem_minmax(0,1fr)] items-center gap-2">
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

	<div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
		<span>
			Aspect is locked to the {$outputSettings.crop ? 'crop' : 'source'} so pixels stay square.
		</span>
		<span>{autoSizeOnUpload ? 'New uploads auto-size to source.' : 'Dimensions are pinned.'}</span>
		<Button size="xs" variant="outline" onclick={resetAspectToSourceOrCrop} disabled={!hasImage}>
			Reset aspect to {$outputSettings.crop ? 'crop' : 'source'}
		</Button>
		<Button
			size="xs"
			variant="ghost"
			onclick={() => (autoSizeOnUpload = !autoSizeOnUpload)}
			aria-pressed={autoSizeOnUpload}
		>
			{autoSizeOnUpload ? 'Pin dimensions' : 'Auto-size new uploads'}
		</Button>
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
