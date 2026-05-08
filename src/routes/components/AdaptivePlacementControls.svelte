<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { Slider } from '$lib/components/ui/slider';
	import type { DitherPlacement } from '$lib/processing/types';
	import { PLACEMENT_MODES } from './output-options';

	type Props = {
		placement: DitherPlacement;
		placementRadius: number;
		placementThreshold: number;
		placementSoftness: number;
		supportsPlacement: boolean;
		placementRadiusMin: number;
		placementRadiusMax: number;
		placementPercentMin: number;
		placementPercentMax: number;
	};

	let {
		placement = $bindable(),
		placementRadius = $bindable(),
		placementThreshold = $bindable(),
		placementSoftness = $bindable(),
		supportsPlacement,
		placementRadiusMin,
		placementRadiusMax,
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
		placementRadius = clampNumber(placementRadius, placementRadiusMin, placementRadiusMax);
		placementThreshold = clampNumber(placementThreshold, placementPercentMin, placementPercentMax);
		placementSoftness = clampNumber(placementSoftness, placementPercentMin, placementPercentMax);
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
	<Label for="dither-placement-radius" class="text-xs text-muted-foreground">Radius</Label>
	<Slider
		type="single"
		bind:value={placementRadius}
		min={placementRadiusMin}
		max={placementRadiusMax}
		step={1}
		disabled={!supportsPlacement || placement !== 'adaptive'}
		aria-label="Adaptive dither radius"
	/>
	<div class="relative">
		<input
			id="dither-placement-radius"
			class="h-8 w-full border border-input bg-background px-2 pr-7 text-right font-mono text-xs tabular-nums"
			type="number"
			min={placementRadiusMin}
			max={placementRadiusMax}
			step="1"
			bind:value={placementRadius}
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
	<Label for="dither-placement-threshold" class="text-xs text-muted-foreground">Threshold</Label>
	<Slider
		type="single"
		bind:value={placementThreshold}
		min={placementPercentMin}
		max={placementPercentMax}
		step={1}
		disabled={!supportsPlacement || placement !== 'adaptive'}
		aria-label="Adaptive dither threshold"
	/>
	<div class="relative">
		<input
			id="dither-placement-threshold"
			class="h-8 w-full border border-input bg-background px-2 pr-5 text-right font-mono text-xs tabular-nums"
			type="number"
			min={placementPercentMin}
			max={placementPercentMax}
			step="1"
			bind:value={placementThreshold}
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

<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
	<Label for="dither-placement-softness" class="text-xs text-muted-foreground">Softness</Label>
	<Slider
		type="single"
		bind:value={placementSoftness}
		min={placementPercentMin}
		max={placementPercentMax}
		step={1}
		disabled={!supportsPlacement || placement !== 'adaptive'}
		aria-label="Adaptive dither softness"
	/>
	<div class="relative">
		<input
			id="dither-placement-softness"
			class="h-8 w-full border border-input bg-background px-2 pr-5 text-right font-mono text-xs tabular-nums"
			type="number"
			min={placementPercentMin}
			max={placementPercentMax}
			step="1"
			bind:value={placementSoftness}
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
