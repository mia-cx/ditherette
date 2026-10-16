<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { Slider } from '$lib/components/ui/slider';
	import type { DitherPlacement } from '$lib/processing/types';
	import { PLACEMENT_MODES } from './output-options';

	type Props = {
		placement: DitherPlacement;
		placementSampleSize: number;
		placementContrastThreshold: number;
		supportsPlacement: boolean;
		placementSampleSizeMin: number;
		placementSampleSizeMax: number;
		placementPercentMin: number;
		placementPercentMax: number;
	};

	let {
		placement = $bindable(),
		placementSampleSize = $bindable(),
		placementContrastThreshold = $bindable(),
		supportsPlacement,
		placementSampleSizeMin,
		placementSampleSizeMax,
		placementPercentMin,
		placementPercentMax
	}: Props = $props();

	const placementLabel = $derived(
		PLACEMENT_MODES.find((option) => option.id === placement)?.label ?? 'Placement'
	);

	function clampNumber(value: number, min: number, max: number) {
		if (!Number.isFinite(value)) return min;
		return Math.min(max, Math.max(min, value));
	}

	function clampPlacementControls() {
		placementSampleSize = clampNumber(
			placementSampleSize,
			placementSampleSizeMin,
			placementSampleSizeMax
		);
		placementContrastThreshold = clampNumber(
			placementContrastThreshold,
			placementPercentMin,
			placementPercentMax
		);
	}
</script>

<div class="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-2">
	<Label for="dither-placement" class="text-xs text-muted-foreground">Placement</Label>
	<Select bind:value={placement} type="single" disabled={!supportsPlacement}>
		<SelectTrigger id="dither-placement" class="w-full">{placementLabel}</SelectTrigger>
		<SelectContent>
			{#each PLACEMENT_MODES as option (option.id)}
				<SelectItem value={option.id}>{option.label}</SelectItem>
			{/each}
		</SelectContent>
	</Select>
</div>

<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
	<Label for="dither-placement-sample-size" class="text-xs text-muted-foreground">Sample size</Label
	>
	<Slider
		type="single"
		bind:value={placementSampleSize}
		min={placementSampleSizeMin}
		max={placementSampleSizeMax}
		step={1}
		disabled={!supportsPlacement || placement !== 'adaptive'}
		aria-label="Adaptive sample size"
	/>
	<div class="relative">
		<input
			id="dither-placement-sample-size"
			class="h-8 w-full border border-input bg-background px-2 pr-7 text-right font-mono text-xs tabular-nums"
			type="number"
			min={placementSampleSizeMin}
			max={placementSampleSizeMax}
			step="1"
			bind:value={placementSampleSize}
			disabled={!supportsPlacement || placement !== 'adaptive'}
			onblur={clampPlacementControls}
			onchange={clampPlacementControls}
		/>
		<span
			class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
			>px</span
		>
	</div>
</div>

<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
	<Label for="dither-placement-contrast-tolerance" class="text-xs text-muted-foreground"
		>Contrast tolerance</Label
	>
	<Slider
		type="single"
		bind:value={placementContrastThreshold}
		min={placementPercentMin}
		max={placementPercentMax}
		step={1}
		disabled={!supportsPlacement || placement !== 'adaptive'}
		aria-label="Adaptive contrast tolerance"
	/>
	<div class="relative">
		<input
			id="dither-placement-contrast-tolerance"
			class="h-8 w-full border border-input bg-background px-2 pr-5 text-right font-mono text-xs tabular-nums"
			type="number"
			min={placementPercentMin}
			max={placementPercentMax}
			step="1"
			bind:value={placementContrastThreshold}
			disabled={!supportsPlacement || placement !== 'adaptive'}
			onblur={clampPlacementControls}
			onchange={clampPlacementControls}
		/>
		<span
			class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
			>%</span
		>
	</div>
</div>
