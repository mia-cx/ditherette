<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Slider } from '$lib/components/ui/slider';
	import { Separator } from '$lib/components/ui/separator';
	import { Button } from '$lib/components/ui/button';
	import { DITHER_ALGORITHMS } from './dither-options';
	import AdaptivePlacementControls from './AdaptivePlacementControls.svelte';
	import DitherAlgorithmSelect from './DitherAlgorithmSelect.svelte';
	import DitherToggleControls from './DitherToggleControls.svelte';
	import { ditherSettings, updateDitherSettings } from '$lib/stores/app';
	import DiceIcon from 'phosphor-svelte/lib/DiceFive';

	type Props = { compact?: boolean; hideHeading?: boolean };
	let { compact = false, hideHeading = false }: Props = $props();

	const PLACEMENT_RADIUS_MIN = 1;
	const PLACEMENT_RADIUS_MAX = 32;
	const PLACEMENT_PERCENT_MIN = 0;
	const PLACEMENT_PERCENT_MAX = 100;
	const initial = ditherSettings.get();
	let algorithm = $state(initial.algorithm);
	let strength = $state<number>(initial.strength);
	let placement = $state(
		initial.placement ?? (initial.coverage === 'full' ? 'everywhere' : 'adaptive')
	);
	let placementRadius = $state(initial.placementRadius ?? 3);
	let placementThreshold = $state(initial.placementThreshold ?? 12);
	let placementSoftness = $state(initial.placementSoftness ?? 8);
	let serpentine = $state(initial.serpentine);
	let seed = $state(initial.seed);
	let useColorSpace = $state(initial.useColorSpace ?? false);
	const current = $derived(DITHER_ALGORITHMS.find((a) => a.id === algorithm));
	const isErrorDiffusion = $derived(current?.family === 'error-diffusion');
	const isNone = $derived(algorithm === 'none');
	const isRandom = $derived(algorithm === 'random');
	const isMixing = $derived(current?.method === 'mixing');
	const isThresholdDither = $derived(current?.family === 'ordered' || current?.family === 'noise');
	const supportsPlacement = $derived(!isNone && (isThresholdDither || isErrorDiffusion));
	const supportsColorSpaceDither = $derived(!isNone && !isMixing);

	// Menus change settings too; follow the store so the controls show what will run.
	$effect(() =>
		ditherSettings.subscribe((settings) => {
			algorithm = settings.algorithm;
			strength = settings.strength;
			placement = settings.placement ?? (settings.coverage === 'full' ? 'everywhere' : 'adaptive');
			placementRadius = settings.placementRadius ?? 3;
			placementThreshold = settings.placementThreshold ?? 12;
			placementSoftness = settings.placementSoftness ?? 8;
			serpentine = settings.serpentine;
			seed = settings.seed;
			useColorSpace = settings.useColorSpace ?? false;
		})
	);

	$effect(() => {
		updateDitherSettings({
			algorithm,
			strength,
			placement,
			placementRadius,
			placementThreshold,
			placementSoftness,
			serpentine,
			seed,
			useColorSpace
		});
	});

	function randomizeSeed() {
		seed = crypto.getRandomValues(new Uint32Array(1))[0];
	}
</script>

<section
	class={compact ? 'flex flex-col gap-3' : 'flex flex-col gap-4'}
	aria-label="Dithering controls"
>
	{#if !hideHeading}
		<div class="flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold tracking-tight">Dithering</h2>
			<p class="text-xs text-muted-foreground">Optional. Off by default.</p>
		</div>
	{/if}

	<div class="grid gap-3">
		<DitherAlgorithmSelect bind:algorithm />

		<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
			<Label for="dither-strength" class="text-xs text-muted-foreground">Strength</Label>
			<Slider
				type="single"
				bind:value={strength}
				min={0}
				max={100}
				step={1}
				disabled={isNone || isMixing}
				aria-label="Dither strength"
			/>
			<div class="relative">
				<input
					id="dither-strength"
					class="h-8 w-full border border-input bg-background px-2 pr-5 text-right font-mono text-xs tabular-nums"
					type="number"
					min="0"
					max="100"
					step="1"
					bind:value={strength}
					disabled={isNone || isMixing}
				/>
				<span
					class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
					>%</span
				>
			</div>
		</div>

		<Separator />

		<AdaptivePlacementControls
			bind:placement
			bind:placementRadius
			bind:placementThreshold
			bind:placementSoftness
			{supportsPlacement}
			placementRadiusMin={PLACEMENT_RADIUS_MIN}
			placementRadiusMax={PLACEMENT_RADIUS_MAX}
			placementPercentMin={PLACEMENT_PERCENT_MIN}
			placementPercentMax={PLACEMENT_PERCENT_MAX}
		/>

		<Separator />

		<DitherToggleControls
			bind:useColorSpace
			bind:serpentine
			{supportsColorSpaceDither}
			{isErrorDiffusion}
		/>

		{#if isRandom}
			<div class="flex items-center justify-between gap-2">
				<div class="flex flex-col gap-0.5">
					<span class="text-sm font-medium">Random seed</span>
					<span class="font-mono text-xs text-muted-foreground"
						>0x{seed.toString(16).padStart(8, '0').toUpperCase()}</span
					>
				</div>
				<Button variant="outline" size="sm" onclick={randomizeSeed}>
					<DiceIcon weight="bold" />
					Randomize
				</Button>
			</div>
		{/if}
	</div>
</section>
