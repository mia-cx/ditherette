<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Slider } from '$lib/components/ui/slider';
	import { Switch } from '$lib/components/ui/switch';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { DITHER_ALGORITHMS, COVERAGE_MODES } from './sample-data';
	import { ditherSettings, updateDitherSettings } from '$lib/stores/app';
	import DiceIcon from 'phosphor-svelte/lib/DiceFive';

	type Props = { compact?: boolean; hideHeading?: boolean };
	let { compact = false, hideHeading = false }: Props = $props();

	const initial = ditherSettings.get();
	let algorithm = $state(initial.algorithm);
	let strength = $state<number>(initial.strength);
	let coverage = $state(initial.coverage);
	let serpentine = $state(initial.serpentine);
	let seed = $state(initial.seed);

	const current = $derived(DITHER_ALGORITHMS.find((a) => a.id === algorithm));
	const isErrorDiffusion = $derived(current?.family === 'error-diffusion');
	const isNone = $derived(algorithm === 'none');
	const isRandom = $derived(algorithm === 'random');
	const triggerLabel = $derived(current?.label ?? 'Select algorithm');
	const coverageLabel = $derived(
		COVERAGE_MODES.find((c) => c.id === coverage)?.label ?? 'Coverage'
	);

	$effect(() => {
		updateDitherSettings({ algorithm, strength, coverage, serpentine, seed });
	});

	function randomizeSeed() {
		seed = crypto.getRandomValues(new Uint32Array(1))[0];
	}
</script>

<section class="flex flex-col gap-{compact ? '3' : '4'}" aria-label="Dithering controls">
	{#if !hideHeading}
		<div class="flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold tracking-tight">Dithering</h2>
			<p class="text-xs text-muted-foreground">Optional. Off by default.</p>
		</div>
	{/if}

	<div class="grid gap-3">
		<div class="grid gap-1.5">
			<Label for="dither-algorithm">Algorithm</Label>
			<Select bind:value={algorithm} type="single">
				<SelectTrigger id="dither-algorithm">{triggerLabel}</SelectTrigger>
				<SelectContent>
					{#each DITHER_ALGORITHMS as opt (opt.id)}
						<SelectItem value={opt.id}>{opt.label}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
		</div>

		<div class="grid gap-1.5">
			<div class="flex items-center justify-between">
				<Label for="dither-strength">Strength</Label>
				<span class="text-xs text-muted-foreground tabular-nums">{strength}%</span>
			</div>
			<Slider
				type="single"
				bind:value={strength}
				min={0}
				max={100}
				step={1}
				disabled={isNone}
				aria-label="Dither strength"
			/>
		</div>

		<div class="grid gap-1.5">
			<Label for="dither-coverage">Coverage</Label>
			<Select bind:value={coverage} type="single" disabled={isNone}>
				<SelectTrigger id="dither-coverage">{coverageLabel}</SelectTrigger>
				<SelectContent>
					{#each COVERAGE_MODES as opt (opt.id)}
						<SelectItem value={opt.id}>{opt.label}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
		</div>

		<div class="flex items-center justify-between gap-2">
			<Label for="dither-serpentine" class="flex flex-col gap-0.5">
				<span>Serpentine scan</span>
				<span class="text-xs font-normal text-muted-foreground"
					>Reduces directional bias for error diffusion.</span
				>
			</Label>
			<Switch id="dither-serpentine" bind:checked={serpentine} disabled={!isErrorDiffusion} />
		</div>

		{#if isRandom}
			<div class="flex items-center justify-between gap-2">
				<div class="flex flex-col gap-0.5">
					<span class="text-sm font-medium">Random seed</span>
					<span class="font-mono text-xs text-muted-foreground"
						>0x{seed.toString(16).padStart(8, '0').toUpperCase()}</span
					>
				</div>
				<Button variant="outline" size="sm" onclick={randomizeSeed}>
					<DiceIcon />
					Randomize
				</Button>
			</div>
		{/if}
	</div>

	<div class="flex items-center gap-3 border border-border bg-background/50 p-2">
		<div
			class="size-16 shrink-0 bg-muted [background-image:repeating-linear-gradient(45deg,transparent_0_3px,rgba(0,0,0,0.18)_3px_4px)]"
			aria-hidden="true"
		></div>
		<div class="min-w-0 flex-1">
			<div class="flex flex-wrap items-center gap-1.5">
				<span class="text-sm font-medium">{current?.label}</span>
				<Badge variant="secondary" class="capitalize">{current?.family.replace('-', ' ')}</Badge>
			</div>
			<p class="mt-0.5 text-xs text-muted-foreground">{current?.short}</p>
		</div>
	</div>
</section>
