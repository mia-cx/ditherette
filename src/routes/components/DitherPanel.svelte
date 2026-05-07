<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Slider } from '$lib/components/ui/slider';
	import { Switch } from '$lib/components/ui/switch';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import {
		Select,
		SelectContent,
		SelectItem,
		SelectTrigger,
	} from '$lib/components/ui/select';
	import { DITHER_ALGORITHMS, COVERAGE_MODES } from './sample-data';
	import DiceIcon from 'phosphor-svelte/lib/DiceFive';

	type Props = {
		compact?: boolean;
		hideHeading?: boolean;
	};

	let { compact = false, hideHeading = false }: Props = $props();

	// Defaults match the spec (algorithm = 'none' off; strength retained
	// at 100 for when the user enables a real algorithm). The collapsed
	// accordion trigger badge in `+page.svelte` reads "Off" — keep this
	// initial state aligned with that summary.
	let algorithm = $state('none');
	let strength = $state<number>(100);
	let coverage = $state('full');
	let serpentine = $state(true);

	const current = $derived(DITHER_ALGORITHMS.find((a) => a.id === algorithm));
	const isErrorDiffusion = $derived(current?.family === 'error-diffusion');
	const isNone = $derived(algorithm === 'none');
	const isRandom = $derived(algorithm === 'random');
	const triggerLabel = $derived(current?.label ?? 'Select algorithm');
	const coverageLabel = $derived(COVERAGE_MODES.find((c) => c.id === coverage)?.label ?? 'Coverage');
</script>

<section
	class="flex flex-col gap-{compact ? '3' : '4'}"
	aria-label="Dithering controls"
>
	{#if !hideHeading}
		<div class="flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold tracking-tight">Dithering</h2>
			<p class="text-muted-foreground text-xs">Optional. Off by default.</p>
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
				<span class="text-muted-foreground text-xs tabular-nums">{strength}%</span>
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
				<span class="text-muted-foreground text-xs font-normal">
					Reduces directional bias for error diffusion.
				</span>
			</Label>
			<Switch id="dither-serpentine" bind:checked={serpentine} disabled={!isErrorDiffusion} />
		</div>

		{#if isRandom}
			<div class="flex items-center justify-between gap-2">
				<div class="flex flex-col gap-0.5">
					<span class="text-sm font-medium">Random seed</span>
					<span class="text-muted-foreground font-mono text-xs">0xC0FFEE42</span>
				</div>
				<Button variant="outline" size="sm">
					<DiceIcon />
					Randomize
				</Button>
			</div>
		{/if}
	</div>

	<div class="border-border bg-background/50 flex items-center gap-3 border p-2">
		<div
			class="bg-muted size-16 shrink-0 [background-image:repeating-linear-gradient(45deg,transparent_0_3px,rgba(0,0,0,0.18)_3px_4px)]"
			aria-hidden="true"
		></div>
		<div class="min-w-0 flex-1">
			<div class="flex flex-wrap items-center gap-1.5">
				<span class="text-sm font-medium">{current?.label}</span>
				<Badge variant="secondary" class="capitalize">{current?.family.replace('-', ' ')}</Badge>
			</div>
			<p class="text-muted-foreground mt-0.5 text-xs">{current?.short}</p>
		</div>
	</div>
</section>
