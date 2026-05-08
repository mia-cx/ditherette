<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Slider } from '$lib/components/ui/slider';
	import { Separator } from '$lib/components/ui/separator';
	import { Button } from '$lib/components/ui/button';
	import { bayerSizeForAlgorithm, normalizedBayerThresholdMatrix } from '$lib/processing/bayer';
	import { clampByte, createPaletteMatcher, vectorForRgb } from '$lib/processing/color';
	import type { ColorSpaceId, EnabledPaletteColor, Rgb } from '$lib/processing/types';
	import { DITHER_ALGORITHMS } from './dither-options';
	import AdaptivePlacementControls from './AdaptivePlacementControls.svelte';
	import DitherAlgorithmSelect from './DitherAlgorithmSelect.svelte';
	import DitherToggleControls from './DitherToggleControls.svelte';
	import { DEFAULT_DITHER_PREVIEW_GRADIENT } from './preview-gradients';
	import { ditherSettings, updateDitherSettings } from '$lib/stores/app';
	import DiceIcon from 'phosphor-svelte/lib/DiceFive';

	type Props = { compact?: boolean; hideHeading?: boolean };
	let { compact = false, hideHeading = false }: Props = $props();

	const DITHER_PREVIEW_PIXEL_SCALE = 4;
	const COLOR_SPACE_THRESHOLD_SCALE = 0.25;
	const PLACEMENT_RADIUS_MIN = 1;
	const PLACEMENT_RADIUS_MAX = 32;
	const PLACEMENT_PERCENT_MIN = 0;
	const PLACEMENT_PERCENT_MAX = 100;
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
	const isThresholdDither = $derived(current?.family === 'ordered' || current?.family === 'noise');
	const supportsPlacement = $derived(!isNone && (isThresholdDither || isErrorDiffusion));
	const supportsColorSpaceDither = $derived(!isNone);

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

	function ditherPreview(
		target: HTMLCanvasElement,
		params: {
			mode: string;
			randomSeed: number;
			previewStrength: number;
			serpentineScan: boolean;
			palette: readonly EnabledPaletteColor[];
			colorSpaceMode: ColorSpaceId;
			useColorSpace: boolean;
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
				next.colorSpaceMode,
				next.useColorSpace
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
		colorSpaceMode: ColorSpaceId,
		thresholdInColorSpace: boolean
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
			colorSpaceMode,
			thresholdInColorSpace
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
		colorSpaceMode: ColorSpaceId,
		thresholdInColorSpace: boolean
	) {
		const matcher = createPaletteMatcher([...palette], colorSpaceMode);
		const amount = Math.min(1, Math.max(0, previewStrength / 100));
		if (mode === 'none') return drawGradientPreview(size, matcher.nearestRgb);
		const kernel = errorKernelFor(mode);
		if (kernel)
			return drawErrorDiffusionPreview(kernel, size, amount, serpentineScan, matcher.nearestRgb);
		return drawThresholdPreview(
			mode,
			size,
			randomSeed,
			amount,
			matcher.nearestRgb,
			[...palette],
			colorSpaceMode,
			thresholdInColorSpace
		);
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
		nearestRgb: PaletteNearest,
		palette: EnabledPaletteColor[],
		colorSpaceMode: ColorSpaceId,
		thresholdInColorSpace: boolean
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
					chooseOrderedPreviewRgb(
						source,
						ditherThreshold,
						nearestRgb,
						amount,
						palette,
						colorSpaceMode,
						thresholdInColorSpace
					)
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

	function previewVectorSpace(palette: EnabledPaletteColor[], colorSpaceMode: ColorSpaceId) {
		const colors = palette
			.filter((color) => color.rgb && color.kind !== 'transparent')
			.map((color) => ({
				rgb: color.rgb!,
				vector: vectorForRgb(color.rgb!.r, color.rgb!.g, color.rgb!.b, colorSpaceMode)
			}));
		const channelRange = (channel: 0 | 1 | 2) => {
			const values = colors.map((color) => color.vector[channel]);
			return Math.max(Math.max(...values) - Math.min(...values), Number.EPSILON);
		};
		return { colors, ranges: [channelRange(0), channelRange(1), channelRange(2)] as ColorVector };
	}

	function nearestPreviewVectorRgb(
		vector: ColorVector,
		space: ReturnType<typeof previewVectorSpace>
	) {
		let winner: Rgb | undefined;
		let best = Infinity;
		for (const candidate of space.colors) {
			const dx = vector[0] - candidate.vector[0];
			const dy = vector[1] - candidate.vector[1];
			const dz = vector[2] - candidate.vector[2];
			const distance = dx * dx + dy * dy + dz * dz;
			if (distance < best) {
				best = distance;
				winner = candidate.rgb;
			}
		}
		return winner;
	}

	function chooseOrderedPreviewRgb(
		rgb: Rgb,
		threshold: number,
		nearestRgb: PaletteNearest,
		strengthAmount: number,
		palette: EnabledPaletteColor[],
		colorSpaceMode: ColorSpaceId,
		thresholdInColorSpace: boolean
	) {
		const offset = (threshold - 0.5) * 192 * strengthAmount;
		if (!thresholdInColorSpace) {
			return nearestPaletteRgb(
				{ r: rgb.r + offset, g: rgb.g + offset, b: rgb.b + offset },
				nearestRgb
			);
		}
		const space = previewVectorSpace(palette, colorSpaceMode);
		const source = vectorForRgb(
			clampByte(rgb.r),
			clampByte(rgb.g),
			clampByte(rgb.b),
			colorSpaceMode
		);
		const amount = (offset / 192) * COLOR_SPACE_THRESHOLD_SCALE;
		const target: ColorVector = [
			source[0] + amount * space.ranges[0],
			source[1] + amount * space.ranges[1],
			source[2] + amount * space.ranges[2]
		];
		return nearestPreviewVectorRgb(target, space) ?? nearestPaletteRgb(rgb, nearestRgb);
	}

	function gradientRgb(x: number, y: number, size: number): Rgb {
		const extent = Math.max(1, size - 1);
		const tx = x / extent;
		const ty = y / extent;
		const position = (tx + (1 - ty)) / 2;
		const stops = DEFAULT_DITHER_PREVIEW_GRADIENT.stops;
		const next = stops.findIndex((stop) => stop.position >= position);
		const high = stops[Math.max(1, next)]!;
		const low = stops[Math.max(0, next - 1)]!;
		const span = Math.max(Number.EPSILON, high.position - low.position);
		const amount = (position - low.position) / span;
		return {
			r: mix(low.color.r, high.color.r, amount),
			g: mix(low.color.g, high.color.g, amount),
			b: mix(low.color.b, high.color.b, amount)
		};
	}

	function mix(start: number, end: number, amount: number) {
		return start + (end - start) * amount;
	}

	function nearestPaletteRgb(rgb: Rgb, nearestRgb: PaletteNearest): Rgb {
		return nearestRgb(clampByte(rgb.r), clampByte(rgb.g), clampByte(rgb.b)).color.rgb!;
	}

	function writePreviewRgb(image: ImageData, x: number, y: number, rgb: Rgb) {
		const offset = (y * image.width + x) * 4;
		image.data[offset] = rgb.r;
		image.data[offset + 1] = rgb.g;
		image.data[offset + 2] = rgb.b;
		image.data[offset + 3] = 255;
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
		<DitherAlgorithmSelect
			bind:algorithm
			{seed}
			{strength}
			{serpentine}
			{useColorSpace}
			{ditherPreview}
		/>

		<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
			<Label for="dither-strength" class="text-xs text-muted-foreground">Strength</Label>
			<Slider
				type="single"
				bind:value={strength}
				min={0}
				max={100}
				step={1}
				disabled={isNone}
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
					disabled={isNone}
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
