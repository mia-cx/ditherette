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

	const DITHER_PREVIEW_PIXEL_SCALE = 3;

	const initial = ditherSettings.get();
	let canvas = $state<HTMLCanvasElement>();
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

	$effect(() => {
		if (!canvas) return;
		drawDitherPreview(canvas, algorithm, seed);
	});

	function randomizeSeed() {
		seed = crypto.getRandomValues(new Uint32Array(1))[0];
	}

	function drawDitherPreview(target: HTMLCanvasElement, mode: string, randomSeed: number) {
		const scale = window.devicePixelRatio || 1;
		const displaySize = Math.max(1, Math.round(target.clientWidth * scale));
		const logicalSize = Math.max(1, Math.round(target.clientWidth / DITHER_PREVIEW_PIXEL_SCALE));
		if (target.width !== displaySize || target.height !== displaySize) {
			target.width = displaySize;
			target.height = displaySize;
		}
		const context = target.getContext('2d');
		if (!context) return;
		const image = new ImageData(logicalSize, logicalSize);
		const random = mulberry32(randomSeed);
		for (let y = 0; y < logicalSize; y++) {
			for (let x = 0; x < logicalSize; x++) {
				const gradient = x / Math.max(1, logicalSize - 1);
				const threshold = thresholdFor(mode, x, y, random);
				const on = gradient + threshold > 0.5;
				const offset = (y * logicalSize + x) * 4;
				const value = on ? 245 : 35;
				image.data[offset] = value;
				image.data[offset + 1] = value;
				image.data[offset + 2] = value;
				image.data[offset + 3] = 255;
			}
		}

		const source = document.createElement('canvas');
		source.width = logicalSize;
		source.height = logicalSize;
		source.getContext('2d')?.putImageData(image, 0, 0);
		context.imageSmoothingEnabled = true;
		context.imageSmoothingQuality = 'high';
		context.clearRect(0, 0, displaySize, displaySize);
		context.drawImage(source, 0, 0, displaySize, displaySize);
	}

	function thresholdFor(mode: string, x: number, y: number, random: () => number) {
		if (mode === 'none') return 0;
		if (mode === 'random') return (random() - 0.5) * 0.45;
		if (mode === 'floyd-steinberg') return ((x + y * 2) % 5) / 12 - 0.16;
		if (mode === 'sierra') return ((x * 3 + y * 5) % 11) / 22 - 0.23;
		if (mode === 'sierra-lite') return ((x * 2 + y * 3) % 7) / 16 - 0.2;
		const size = mode === 'bayer-4' ? 4 : mode === 'bayer-8' ? 8 : 16;
		return (bayerValue(size, x % size, y % size) - 0.5) * 0.55;
	}

	function bayerValue(size: number, x: number, y: number): number {
		let value = 0;
		for (let bit = size >> 1; bit > 0; bit >>= 1) {
			const quadrant = (x & bit ? 1 : 0) + (y & bit ? 2 : 0);
			value = value * 4 + [0, 2, 3, 1][quadrant]!;
		}
		return (value + 0.5) / (size * size);
	}

	function mulberry32(value: number) {
		let state = value >>> 0;
		return () => {
			state += 0x6d2b79f5;
			let next = state;
			next = Math.imul(next ^ (next >>> 15), next | 1);
			next ^= next + Math.imul(next ^ (next >>> 7), next | 61);
			return ((next ^ (next >>> 14)) >>> 0) / 4294967296;
		};
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
		<canvas
			bind:this={canvas}
			class="size-24 shrink-0 bg-muted [image-rendering:auto]"
			aria-label="{current?.label} deterministic dither preview"
		></canvas>
		<div class="min-w-0 flex-1">
			<div class="flex flex-wrap items-center gap-1.5">
				<span class="text-sm font-medium">{current?.label}</span>
				<Badge variant="secondary" class="capitalize">{current?.family.replace('-', ' ')}</Badge>
			</div>
			<p class="mt-0.5 text-xs text-muted-foreground">{current?.short}</p>
		</div>
	</div>
</section>
