<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { Slider } from '$lib/components/ui/slider';
	import type { DitherPlacement } from '$lib/processing/types';
	import { PLACEMENT_MODES } from './output-options';

	type Props = {
		placement: DitherPlacement;
		placementMinGradientSize: number;
		supportsPlacement: boolean;
		placementMinGradientSizeMin: number;
		placementMinGradientSizeMax: number;
	};

	let {
		placement = $bindable(),
		placementMinGradientSize = $bindable(),
		supportsPlacement,
		placementMinGradientSizeMin,
		placementMinGradientSizeMax
	}: Props = $props();

	const placementLabel = $derived(
		PLACEMENT_MODES.find((option) => option.id === placement)?.label ?? 'Placement'
	);

	function clampNumber(value: number, min: number, max: number) {
		if (!Number.isFinite(value)) return min;
		return Math.min(max, Math.max(min, value));
	}

	function clampPlacementControls() {
		placementMinGradientSize = clampNumber(
			placementMinGradientSize,
			placementMinGradientSizeMin,
			placementMinGradientSizeMax
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
	<Label for="dither-placement-min-gradient" class="text-xs text-muted-foreground"
		>Min gradient</Label
	>
	<Slider
		type="single"
		bind:value={placementMinGradientSize}
		min={placementMinGradientSizeMin}
		max={placementMinGradientSizeMax}
		step={1}
		disabled={!supportsPlacement || placement !== 'adaptive'}
		aria-label="Minimum gradient length"
	/>
	<div class="relative">
		<input
			id="dither-placement-min-gradient"
			class="h-8 w-full border border-input bg-background px-2 pr-7 text-right font-mono text-xs tabular-nums"
			type="number"
			min={placementMinGradientSizeMin}
			max={placementMinGradientSizeMax}
			step="1"
			bind:value={placementMinGradientSize}
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
