<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Slider } from '$lib/components/ui/slider';
	import { Switch } from '$lib/components/ui/switch';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { bayerSizeForAlgorithm, normalizedBayerThresholdMatrix } from '$lib/processing/bayer';
	import { clampByte, createPaletteMatcher, vectorForRgb } from '$lib/processing/color';
	import type { ColorSpaceId, EnabledPaletteColor, Rgb } from '$lib/processing/types';
	import { DITHER_ALGORITHMS, COVERAGE_MODES, type DitherOption } from './sample-data';
	import {
		colorSpace,
		ditherSettings,
		selectedPalette,
		updateDitherSettings
	} from '$lib/stores/app';
	import DiceIcon from 'phosphor-svelte/lib/DiceFive';

	type Props = { compact?: boolean; hideHeading?: boolean };
	let { compact = false, hideHeading = false }: Props = $props();

	const DITHER_PREVIEW_PIXEL_SCALE = 2;
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

	function ditherPreview(
		target: HTMLCanvasElement,
		params: {
			mode: string;
			randomSeed: number;
			previewStrength: number;
			serpentineScan: boolean;
			palette: readonly EnabledPaletteColor[];
			colorSpaceMode: ColorSpaceId;
		}
	) {
		const draw = (next: typeof params) => {
			drawDitherPreview(
				target,
				next.mode,
				next.randomSeed,
				next.previewStrength,
				next.serpentineScan,
				next.palette,
				next.colorSpaceMode
			);
		};
		draw(params);
		return { update: draw };
	}

	function drawDitherPreview(
		target: HTMLCanvasElement,
		mode: string,
		randomSeed: number,
		previewStrength: number,
		serpentineScan: boolean,
		palette: readonly EnabledPaletteColor[],
		colorSpaceMode: ColorSpaceId
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
		const image = drawPreviewImage(
			mode,
			logicalSize,
			randomSeed,
			previewStrength,
			serpentineScan,
			palette,
			colorSpaceMode
		);

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
		serpentineScan: boolean,
		palette: readonly EnabledPaletteColor[],
		colorSpaceMode: ColorSpaceId
	) {
		const matcher = createPaletteMatcher([...palette], colorSpaceMode);
		const amount = Math.min(1, Math.max(0, previewStrength / 100));
		if (mode === 'none') return drawGradientPreview(size, matcher.nearestRgb);
		const kernel = errorKernelFor(mode);
		if (kernel)
			return drawErrorDiffusionPreview(kernel, size, amount, serpentineScan, matcher.nearestRgb);
		return drawThresholdPreview(mode, size, randomSeed, amount, matcher.nearestRgb);
	}

	function drawGradientPreview(size: number, nearestRgb: PaletteNearest) {
		const image = new ImageData(size, size);
		for (let y = 0; y < size; y++) {
			for (let x = 0; x < size; x++) {
				writePreviewRgb(image, x, y, nearestPaletteRgb(gradientRgb(x, y, size), nearestRgb));
			}
		}
		return image;
	}

	function drawThresholdPreview(
		mode: string,
		size: number,
		randomSeed: number,
		amount: number,
		nearestRgb: PaletteNearest
	) {
		const image = new ImageData(size, size);
		const random = mulberry32(randomSeed);
		const bayerSize = bayerSizeForAlgorithm(mode);
		const matrix = bayerSize ? normalizedBayerThresholdMatrix(bayerSize) : undefined;
		const matrixSize = bayerSize ?? 1;
		for (let y = 0; y < size; y++) {
			for (let x = 0; x < size; x++) {
				const source = gradientRgb(x, y, size);
				const ditherThreshold = matrix
					? matrix[(y % matrixSize) * matrixSize + (x % matrixSize)]!
					: random();
				writePreviewRgb(
					image,
					x,
					y,
					chooseOrderedPreviewRgb(source, ditherThreshold, nearestRgb, amount)
				);
			}
		}
		return image;
	}

	function drawErrorDiffusionPreview(
		kernel: [number, number, number][],
		size: number,
		amount: number,
		serpentineScan: boolean,
		nearestRgb: PaletteNearest
	) {
		if (amount <= 0) return drawGradientPreview(size, nearestRgb);
		const image = new ImageData(size, size);
		const red = new Float32Array(size * size);
		const green = new Float32Array(size * size);
		const blue = new Float32Array(size * size);
		for (let y = 0; y < size; y++) {
			for (let x = 0; x < size; x++) {
				const index = y * size + x;
				const source = gradientRgb(x, y, size);
				red[index] = source.r;
				green[index] = source.g;
				blue[index] = source.b;
			}
		}

		for (let y = 0; y < size; y++) {
			const reverse = serpentineScan && y % 2 === 1;
			const start = reverse ? size - 1 : 0;
			const end = reverse ? -1 : size;
			const step = reverse ? -1 : 1;
			for (let x = start; x !== end; x += step) {
				const index = y * size + x;
				const source = { r: red[index]!, g: green[index]!, b: blue[index]! };
				const chosen = nearestPaletteRgb(source, nearestRgb);
				writePreviewRgb(image, x, y, chosen);
				const error = {
					r: source.r - chosen.r,
					g: source.g - chosen.g,
					b: source.b - chosen.b
				};
				for (const [dxBase, dy, weight] of kernel) {
					const dx = reverse ? -dxBase : dxBase;
					const xx = x + dx;
					const yy = y + dy;
					if (xx < 0 || xx >= size || yy < 0 || yy >= size) continue;
					const next = yy * size + xx;
					const scaledWeight = weight * amount;
					red[next] += error.r * scaledWeight;
					green[next] += error.g * scaledWeight;
					blue[next] += error.b * scaledWeight;
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

	type PaletteNearest = ReturnType<typeof createPaletteMatcher>['nearestRgb'];
	type ColorVector = ReturnType<typeof vectorForRgb>;

	function vectorDistance(left: ColorVector, right: ColorVector) {
		const dx = left[0] - right[0];
		const dy = left[1] - right[1];
		if ($colorSpace === 'oklch') {
			const hue = Math.atan2(Math.sin(left[2] - right[2]), Math.cos(left[2] - right[2]));
			const dh = Math.min(left[1], right[1]) * hue;
			return dx * dx + dy * dy + dh * dh;
		}
		const dz = left[2] - right[2];
		return dx * dx + dy * dy + dz * dz;
	}

	function nearestPreviewVector(vector: ColorVector) {
		let winner: Rgb | undefined;
		let winnerVector: ColorVector | undefined;
		let best = Number.POSITIVE_INFINITY;
		for (const color of $selectedPalette) {
			if (!color.rgb || color.kind === 'transparent') continue;
			const candidateVector = vectorForRgb(color.rgb.r, color.rgb.g, color.rgb.b, $colorSpace);
			const distance = vectorDistance(vector, candidateVector);
			if (distance < best) {
				best = distance;
				winner = color.rgb;
				winnerVector = candidateVector;
			}
		}
		return { rgb: winner, vector: winnerVector };
	}

	function chooseOrderedPreviewRgb(
		rgb: Rgb,
		threshold: number,
		nearestRgb: PaletteNearest,
		strengthAmount: number
	) {
		if (strengthAmount <= 0)
			return nearestRgb(clampByte(rgb.r), clampByte(rgb.g), clampByte(rgb.b)).color.rgb!;
		const source = vectorForRgb(clampByte(rgb.r), clampByte(rgb.g), clampByte(rgb.b), $colorSpace);
		const nearest = nearestPreviewVector(source);
		if (!nearest.rgb || !nearest.vector)
			return nearestRgb(clampByte(rgb.r), clampByte(rgb.g), clampByte(rgb.b)).color.rgb!;
		const amount = (threshold - 0.5) * 2 * strengthAmount;
		const target: ColorVector = [
			source[0] + (source[0] - nearest.vector[0]) * amount,
			source[1] + (source[1] - nearest.vector[1]) * amount,
			source[2] + (source[2] - nearest.vector[2]) * amount
		];
		return nearestPreviewVector(target).rgb ?? nearest.rgb;
	}

	function gradientRgb(x: number, y: number, size: number): Rgb {
		const extent = Math.max(1, size - 1);
		const tx = x / extent;
		const ty = y / extent;
		const lime = { r: 135, g: 255, b: 94 };
		const yellow = { r: 249, g: 221, b: 59 };
		const violet = { r: 120, g: 12, b: 153 };
		const blue = { r: 40, g: 80, b: 158 };
		return {
			r: mix(mix(lime.r, yellow.r, tx), mix(blue.r, violet.r, tx), ty),
			g: mix(mix(lime.g, yellow.g, tx), mix(blue.g, violet.g, tx), ty),
			b: mix(mix(lime.b, yellow.b, tx), mix(blue.b, violet.b, tx), ty)
		};
	}

	function nearestPaletteRgb(rgb: Rgb, nearestRgb: PaletteNearest): Rgb {
		return nearestRgb(clampByte(rgb.r), clampByte(rgb.g), clampByte(rgb.b)).color.rgb!;
	}

	function mix(start: number, end: number, amount: number) {
		return start + (end - start) * amount;
	}

	function writePreviewRgb(image: ImageData, x: number, y: number, rgb: Rgb) {
		const offset = (y * image.width + x) * 4;
		image.data[offset] = rgb.r;
		image.data[offset + 1] = rgb.g;
		image.data[offset + 2] = rgb.b;
		image.data[offset + 3] = 255;
	}

	function familyLabel(family: DitherOption['family']) {
		return family.replace('-', ' ');
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
		<Select bind:value={algorithm} type="single">
			<SelectTrigger
				id="dither-algorithm"
				size="auto"
				class="w-full items-center gap-3 border-border bg-background/50 p-3 text-left whitespace-normal"
			>
				{#if current}
					<span class="grid min-w-0 flex-1 gap-1.5">
						<span class="flex items-center gap-3">
							<canvas
								use:ditherPreview={{
									mode: current.id,
									randomSeed: seed,
									previewStrength: strength,
									serpentineScan: serpentine,
									palette: $selectedPalette,
									colorSpaceMode: $colorSpace
								}}
								class="size-20 shrink-0 bg-muted [image-rendering:pixelated]"
								aria-hidden="true"
							></canvas>
							<span class="grid min-w-0 flex-1 gap-1">
								<span class="flex min-w-0 items-center gap-1.5">
									<span class="truncate text-sm font-medium text-foreground">{current.label}</span>
									<Badge variant="secondary" class="capitalize">{familyLabel(current.family)}</Badge
									>
								</span>
								<span class="text-xs text-muted-foreground">{current.short}</span>
							</span>
						</span>
					</span>
				{:else}
					{triggerLabel}
				{/if}
			</SelectTrigger>
			<SelectContent class="max-h-[min(34rem,var(--bits-select-content-available-height))] p-1">
				{#each DITHER_ALGORITHMS as opt (opt.id)}
					<SelectItem value={opt.id} label={opt.label} class="items-center py-3 pr-8 pl-3">
						<span class="grid min-w-0 flex-1 gap-1.5">
							<span class="flex items-center gap-3">
								<canvas
									use:ditherPreview={{
										mode: opt.id,
										randomSeed: seed,
										previewStrength: strength,
										serpentineScan: serpentine,
										palette: $selectedPalette,
										colorSpaceMode: $colorSpace
									}}
									class="size-20 shrink-0 bg-muted [image-rendering:pixelated]"
									aria-hidden="true"
								></canvas>
								<span class="grid min-w-0 flex-1 gap-1">
									<span class="flex min-w-0 items-center gap-1.5">
										<span class="truncate text-sm font-medium text-foreground">{opt.label}</span>
										<Badge variant="secondary" class="capitalize">{familyLabel(opt.family)}</Badge>
									</span>
									<span class="text-xs whitespace-normal text-muted-foreground">{opt.short}</span>
								</span>
							</span>
						</span>
					</SelectItem>
				{/each}
			</SelectContent>
		</Select>

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
					<DiceIcon weight="bold" />
					Randomize
				</Button>
			</div>
		{/if}
	</div>
</section>
