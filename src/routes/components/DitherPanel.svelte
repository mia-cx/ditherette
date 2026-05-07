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

	const DITHER_PREVIEW_PIXEL_SCALE = 6;
	const BAYER_4 = makeBayer(4);
	const BAYER_8 = makeBayer(8);
	const BAYER_16 = makeBayer(16);
	const ERROR_KERNELS = {
		'floyd-steinberg': [
			[1, 0, 7 / 16],
			[-1, 1, 3 / 16],
			[0, 1, 5 / 16],
			[1, 1, 1 / 16]
		],
		sierra: [
			[1, 0, 5 / 32],
			[2, 0, 3 / 32],
			[-2, 1, 2 / 32],
			[-1, 1, 4 / 32],
			[0, 1, 5 / 32],
			[1, 1, 4 / 32],
			[2, 1, 2 / 32],
			[-1, 2, 2 / 32],
			[0, 2, 3 / 32],
			[1, 2, 2 / 32]
		],
		'sierra-lite': [
			[1, 0, 2 / 4],
			[-1, 1, 1 / 4],
			[0, 1, 1 / 4]
		]
	} satisfies Record<string, [number, number, number][]>;

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
		drawDitherPreview(canvas, algorithm, seed, strength, serpentine);
	});

	function randomizeSeed() {
		seed = crypto.getRandomValues(new Uint32Array(1))[0];
	}

	function drawDitherPreview(
		target: HTMLCanvasElement,
		mode: string,
		randomSeed: number,
		previewStrength: number,
		serpentineScan: boolean
	) {
		const scale = window.devicePixelRatio || 1;
		const displaySize = Math.max(1, Math.round(target.clientWidth * scale));
		const logicalSize = Math.max(1, Math.round(target.clientWidth / DITHER_PREVIEW_PIXEL_SCALE));
		if (target.width !== displaySize || target.height !== displaySize) {
			target.width = displaySize;
			target.height = displaySize;
		}
		const context = target.getContext('2d');
		if (!context) return;
		const image = drawPreviewImage(mode, logicalSize, randomSeed, previewStrength, serpentineScan);

		const source = document.createElement('canvas');
		source.width = logicalSize;
		source.height = logicalSize;
		source.getContext('2d')?.putImageData(image, 0, 0);
		context.imageSmoothingEnabled = false;
		context.clearRect(0, 0, displaySize, displaySize);
		context.drawImage(source, 0, 0, displaySize, displaySize);
	}

	function drawPreviewImage(
		mode: string,
		size: number,
		randomSeed: number,
		previewStrength: number,
		serpentineScan: boolean
	) {
		const amount = Math.min(1, Math.max(0, previewStrength / 100));
		if (mode === 'none') return drawGradientPreview(size);
		const kernel = errorKernelFor(mode);
		if (kernel) return drawErrorDiffusionPreview(kernel, size, amount, serpentineScan);
		return drawThresholdPreview(mode, size, randomSeed, amount);
	}

	function drawGradientPreview(size: number) {
		const image = new ImageData(size, size);
		for (let y = 0; y < size; y++) {
			for (let x = 0; x < size; x++) {
				writePreviewPixel(image, x, y, gradientValue(x, size));
			}
		}
		return image;
	}

	function drawThresholdPreview(mode: string, size: number, randomSeed: number, amount: number) {
		const image = new ImageData(size, size);
		const random = mulberry32(randomSeed);
		const matrix = bayerMatrixFor(mode);
		const matrixSize = matrix ? Math.sqrt(matrix.length) : 1;
		for (let y = 0; y < size; y++) {
			for (let x = 0; x < size; x++) {
				const gradient = x / Math.max(1, size - 1);
				const ditherThreshold = matrix
					? matrix[(y % matrixSize) * matrixSize + (x % matrixSize)]!
					: random();
				const threshold = 0.5 + (ditherThreshold - 0.5) * amount;
				writePreviewPixel(image, x, y, gradient >= threshold ? 255 : 0);
			}
		}
		return image;
	}

	function drawErrorDiffusionPreview(
		kernel: [number, number, number][],
		size: number,
		amount: number,
		serpentineScan: boolean
	) {
		if (amount <= 0) return drawGradientPreview(size);
		const image = new ImageData(size, size);
		const work = new Float32Array(size * size);
		for (let y = 0; y < size; y++) {
			for (let x = 0; x < size; x++) {
				work[y * size + x] = gradientValue(x, size);
			}
		}

		for (let y = 0; y < size; y++) {
			const reverse = serpentineScan && y % 2 === 1;
			const start = reverse ? size - 1 : 0;
			const end = reverse ? -1 : size;
			const step = reverse ? -1 : 1;
			for (let x = start; x !== end; x += step) {
				const index = y * size + x;
				const value = clampByte(work[index]!);
				const chosen = value >= 128 ? 255 : 0;
				writePreviewPixel(image, x, y, chosen);
				const error = value - chosen;
				for (const [dxBase, dy, weight] of kernel) {
					const dx = reverse ? -dxBase : dxBase;
					const xx = x + dx;
					const yy = y + dy;
					if (xx < 0 || xx >= size || yy < 0 || yy >= size) continue;
					work[yy * size + xx] += error * weight * amount;
				}
			}
		}
		return image;
	}

	function errorKernelFor(mode: string) {
		if (mode === 'floyd-steinberg') return ERROR_KERNELS['floyd-steinberg'];
		if (mode === 'sierra') return ERROR_KERNELS.sierra;
		if (mode === 'sierra-lite') return ERROR_KERNELS['sierra-lite'];
		return undefined;
	}

	function bayerMatrixFor(mode: string) {
		if (mode === 'bayer-4') return BAYER_4;
		if (mode === 'bayer-8') return BAYER_8;
		if (mode === 'bayer-16') return BAYER_16;
		return undefined;
	}

	function gradientValue(x: number, width: number) {
		return Math.round((x / Math.max(1, width - 1)) * 255);
	}

	function clampByte(value: number) {
		return Math.min(255, Math.max(0, value));
	}

	function writePreviewPixel(image: ImageData, x: number, y: number, value: number) {
		const byte = clampByte(Math.round(value));
		const offset = (y * image.width + x) * 4;
		image.data[offset] = byte;
		image.data[offset + 1] = byte;
		image.data[offset + 2] = byte;
		image.data[offset + 3] = 255;
	}

	function makeBayer(size: number): number[] {
		let matrix = [0];
		let current = 1;
		while (current < size) {
			const next = current * 2;
			const output = new Array(next * next).fill(0);
			for (let y = 0; y < current; y++) {
				for (let x = 0; x < current; x++) {
					const value = matrix[y * current + x] * 4;
					output[y * next + x] = value;
					output[y * next + x + current] = value + 2;
					output[(y + current) * next + x] = value + 3;
					output[(y + current) * next + x + current] = value + 1;
				}
			}
			matrix = output;
			current = next;
		}
		return matrix.map((value) => (value + 0.5) / (size * size));
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
			class="size-24 shrink-0 bg-muted [image-rendering:pixelated]"
			aria-label="{current?.label} deterministic dither preview"
		></canvas>
		<div class="min-w-0 flex-1">
			<div class="flex flex-wrap items-center gap-1.5">
				<span class="text-sm font-medium">{current?.label}</span>
				<Badge variant="secondary" class="capitalize">{current?.family.replace('-', ' ')}</Badge>
			</div>
			<p class="mt-0.5 text-xs text-muted-foreground">{current?.short}</p>
			<p
				class="mt-1 border-l-2 border-border bg-muted/50 px-2 py-1 font-mono text-xs text-muted-foreground"
			>
				{current?.math}
			</p>
		</div>
	</div>
</section>
