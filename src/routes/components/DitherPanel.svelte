<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Slider } from '$lib/components/ui/slider';
	import { Switch } from '$lib/components/ui/switch';
	import { Separator } from '$lib/components/ui/separator';
	import { Sheet, SheetContent, SheetHeader, SheetTitle } from '$lib/components/ui/sheet';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Accordion, AccordionContent, AccordionItem } from '$lib/components/ui/accordion';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { bayerSizeForAlgorithm, normalizedBayerThresholdMatrix } from '$lib/processing/bayer';
	import { clampByte, createPaletteMatcher, vectorForRgb } from '$lib/processing/color';
	import type { ColorSpaceId, EnabledPaletteColor, Rgb } from '$lib/processing/types';
	import {
		DITHER_ALGORITHMS,
		PLACEMENT_MODES,
		type DitherField,
		type DitherMethod
	} from './sample-data';
	import {
		colorSpace,
		ditherSettings,
		selectedPalette,
		uiSettings,
		updateDitherSettings
	} from '$lib/stores/app';
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
	let algorithmSearch = $state('');
	let methodFilters = $state<DitherMethod[]>(['none', 'threshold', 'error-diffusion']);
	let fieldFilters = $state<DitherField[]>(['none', 'ordered', 'noise', 'kernel']);
	let filterSheetOpen = $state(false);
	let desktopFilterSections = $state<string[]>(
		uiSettings.get().desktopDitherFiltersOpen ? ['filters'] : []
	);

	const current = $derived(DITHER_ALGORITHMS.find((a) => a.id === algorithm));
	const isErrorDiffusion = $derived(current?.family === 'error-diffusion');
	const isNone = $derived(algorithm === 'none');
	const isRandom = $derived(algorithm === 'random');
	const isThresholdDither = $derived(current?.family === 'ordered' || current?.family === 'noise');
	const supportsPlacement = $derived(!isNone && (isThresholdDither || isErrorDiffusion));
	const supportsColorSpaceDither = $derived(!isNone);
	const triggerLabel = $derived(current?.label ?? 'Select algorithm');
	const placementLabel = $derived(
		PLACEMENT_MODES.find((option) => option.id === placement)?.label ?? 'Placement'
	);
	const filteredAlgorithms = $derived(
		DITHER_ALGORITHMS.filter((option) => {
			const query = algorithmSearch.trim().toLowerCase();
			const matchesQuery =
				query.length === 0 ||
				[option.label, option.short, option.sku, option.method, option.field]
					.join(' ')
					.toLowerCase()
					.includes(query);
			const matchesMethod = methodFilters.includes(option.method);
			const matchesField = fieldFilters.includes(option.field);
			return matchesQuery && matchesMethod && matchesField;
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

	$effect(() => {
		uiSettings.set({
			...uiSettings.get(),
			desktopDitherFiltersOpen: desktopFilterSections.includes('filters')
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
		const dx = x / extent - 0.5;
		const dy = y / extent - 0.5;
		const angle = (Math.atan2(dy, dx) + Math.PI * 2) % (Math.PI * 2);
		const anchors = [
			{ angle: 0, color: { r: 255, g: 127, b: 39 } },
			{ angle: Math.PI / 4, color: { r: 237, g: 28, b: 36 } },
			{ angle: Math.PI / 2, color: { r: 236, g: 31, b: 128 } },
			{ angle: (Math.PI * 3) / 4, color: { r: 170, g: 56, b: 185 } },
			{ angle: Math.PI, color: { r: 40, g: 80, b: 158 } },
			{ angle: (Math.PI * 5) / 4, color: { r: 14, g: 185, b: 104 } },
			{ angle: (Math.PI * 3) / 2, color: { r: 135, g: 255, b: 94 } },
			{ angle: (Math.PI * 7) / 4, color: { r: 249, g: 221, b: 59 } },
			{ angle: Math.PI * 2, color: { r: 255, g: 127, b: 39 } }
		];
		const next = anchors.findIndex((anchor) => anchor.angle >= angle);
		const high = anchors[Math.max(1, next)]!;
		const low = anchors[Math.max(0, next - 1)]!;
		const span = Math.max(Number.EPSILON, high.angle - low.angle);
		const amount = (angle - low.angle) / span;
		const hue = {
			r: mix(low.color.r, high.color.r, amount),
			g: mix(low.color.g, high.color.g, amount),
			b: mix(low.color.b, high.color.b, amount)
		};
		const radius = Math.min(1, Math.hypot(dx, dy) / Math.SQRT1_2);
		const saturation = Math.min(1, radius * 2);
		const value = radius <= 0.5 ? 1 : 1 - (radius - 0.5) * 2;
		return {
			r: mix(255, hue.r * value, saturation),
			g: mix(255, hue.g * value, saturation),
			b: mix(255, hue.b * value, saturation)
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

	function methodLabel(method: DitherMethod) {
		if (method === 'error-diffusion') return 'Error diffusion';
		if (method === 'threshold') return 'Threshold';
		return 'None';
	}

	function fieldLabel(field: DitherField) {
		if (field === 'ordered') return 'Ordered';
		if (field === 'noise') return 'Noise';
		if (field === 'kernel') return 'Kernel';
		return 'None';
	}

	function toggleMethodFilter(method: DitherMethod) {
		methodFilters = methodFilters.includes(method)
			? methodFilters.filter((value) => value !== method)
			: [...methodFilters, method];
	}

	function toggleFieldFilter(field: DitherField) {
		fieldFilters = fieldFilters.includes(field)
			? fieldFilters.filter((value) => value !== field)
			: [...fieldFilters, field];
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
						<span class="flex items-start gap-3">
							<canvas
								use:ditherPreview={{
									mode: current.id,
									randomSeed: seed,
									previewStrength: strength,
									serpentineScan: serpentine,
									palette: $selectedPalette,
									colorSpaceMode: $colorSpace,
									useColorSpace
								}}
								class="size-24 shrink-0 bg-muted [image-rendering:pixelated]"
								aria-hidden="true"
							></canvas>
							<span class="grid min-w-0 flex-1 content-start gap-1">
								<span class="flex min-w-0 flex-wrap items-start gap-1.5">
									<span class="truncate text-sm font-medium text-foreground">{current.label}</span>
									<Badge variant="secondary">{methodLabel(current.method)}</Badge>
									<Badge variant="outline">{fieldLabel(current.field)}</Badge>
								</span>
								<span class="text-xs text-muted-foreground">{current.short}</span>
							</span>
						</span>
					</span>
				{:else}
					{triggerLabel}
				{/if}
			</SelectTrigger>
			<SelectContent
				class="h-[min(38rem,var(--bits-select-content-available-height))] w-[calc(100vw-2rem)] max-w-[calc(100vw-2rem)] p-1 md:w-auto md:max-w-none"
			>
				<div class="sticky top-0 z-10 border-b border-border bg-popover p-2">
					<div class="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] gap-2">
						<input
							class="h-8 min-w-0 border border-input bg-background px-2 text-xs outline-none focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring/50"
							placeholder="Search algorithms…"
							bind:value={algorithmSearch}
							onkeydown={(event) => event.stopPropagation()}
						/>
						<Button
							variant="outline"
							size="default"
							class="md:hidden"
							onclick={() => (filterSheetOpen = true)}>Filters</Button
						>
						<Button
							variant="outline"
							size="default"
							class="hidden md:inline-flex"
							aria-expanded={desktopFilterSections.includes('filters')}
							onclick={() =>
								(desktopFilterSections = desktopFilterSections.includes('filters')
									? []
									: ['filters'])}>Filters</Button
						>
					</div>
				</div>
				<div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_auto] gap-2 overflow-hidden">
					<div class="min-w-0 overflow-y-auto">
						{#each filteredAlgorithms as opt (opt.id)}
							<SelectItem
								value={opt.id}
								label={opt.label}
								class="min-w-0 items-center py-3 pr-8 pl-3"
							>
								<span class="grid min-w-0 flex-1 gap-1.5">
									<span class="flex items-start gap-3">
										<canvas
											use:ditherPreview={{
												mode: opt.id,
												randomSeed: seed,
												previewStrength: strength,
												serpentineScan: serpentine,
												palette: $selectedPalette,
												colorSpaceMode: $colorSpace,
												useColorSpace
											}}
											class="size-20 shrink-0 bg-muted [image-rendering:pixelated] sm:size-24"
											aria-hidden="true"
										></canvas>
										<span class="grid min-w-0 flex-1 content-start gap-1">
											<span class="flex min-w-0 flex-wrap items-start gap-1.5">
												<span class="truncate text-sm font-medium text-foreground">{opt.label}</span
												>
												<Badge variant="secondary">{methodLabel(opt.method)}</Badge>
												<Badge variant="outline">{fieldLabel(opt.field)}</Badge>
											</span>
											<span class="text-xs whitespace-normal text-muted-foreground"
												>{opt.short}</span
											>
										</span>
									</span>
								</span>
							</SelectItem>
						{:else}
							<div class="p-4 text-center text-xs text-muted-foreground">No algorithms match.</div>
						{/each}
					</div>
					<Accordion
						type="multiple"
						bind:value={desktopFilterSections}
						class="hidden h-full overflow-hidden border-l border-border md:flex"
					>
						<AccordionItem value="filters" class="border-b-0">
							<AccordionContent class="w-40 p-2 pb-2.5">
								<div class="grid gap-3">
									<div class="grid gap-1.5">
										<span
											class="text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase"
											>Method</span
										>
										<label class="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Checkbox
												checked={methodFilters.includes('threshold')}
												onCheckedChange={() => toggleMethodFilter('threshold')}
												aria-label="Toggle threshold method filter"
											/>
											Threshold
										</label>
										<label class="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Checkbox
												checked={methodFilters.includes('error-diffusion')}
												onCheckedChange={() => toggleMethodFilter('error-diffusion')}
												aria-label="Toggle error-diffusion method filter"
											/>
											Error diffusion
										</label>
										<label class="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Checkbox
												checked={methodFilters.includes('none')}
												onCheckedChange={() => toggleMethodFilter('none')}
												aria-label="Toggle none method filter"
											/>
											None
										</label>
									</div>
									<div class="grid gap-1.5">
										<span
											class="text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase"
											>Field</span
										>
										<label class="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Checkbox
												checked={fieldFilters.includes('ordered')}
												onCheckedChange={() => toggleFieldFilter('ordered')}
												aria-label="Toggle ordered field filter"
											/>
											Ordered
										</label>
										<label class="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Checkbox
												checked={fieldFilters.includes('noise')}
												onCheckedChange={() => toggleFieldFilter('noise')}
												aria-label="Toggle noise field filter"
											/>
											Noise
										</label>
										<label class="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Checkbox
												checked={fieldFilters.includes('kernel')}
												onCheckedChange={() => toggleFieldFilter('kernel')}
												aria-label="Toggle kernel field filter"
											/>
											Kernel
										</label>
										<label class="flex items-center gap-1.5 text-xs text-muted-foreground">
											<Checkbox
												checked={fieldFilters.includes('none')}
												onCheckedChange={() => toggleFieldFilter('none')}
												aria-label="Toggle none field filter"
											/>
											None
										</label>
									</div>
								</div>
							</AccordionContent>
						</AccordionItem>
					</Accordion>
				</div>
			</SelectContent>
		</Select>

		<Sheet bind:open={filterSheetOpen}>
			<SheetContent side="right" class="w-72 p-4">
				<SheetHeader>
					<SheetTitle>Algorithm filters</SheetTitle>
				</SheetHeader>
				<div class="grid content-start gap-4 pt-4">
					<div class="grid gap-2">
						<span class="text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase"
							>Method</span
						>
						<label class="flex items-center gap-2 text-xs text-muted-foreground">
							<Checkbox
								checked={methodFilters.includes('threshold')}
								onCheckedChange={() => toggleMethodFilter('threshold')}
								aria-label="Toggle threshold method filter"
							/>
							Threshold
						</label>
						<label class="flex items-center gap-2 text-xs text-muted-foreground">
							<Checkbox
								checked={methodFilters.includes('error-diffusion')}
								onCheckedChange={() => toggleMethodFilter('error-diffusion')}
								aria-label="Toggle error-diffusion method filter"
							/>
							Error diffusion
						</label>
						<label class="flex items-center gap-2 text-xs text-muted-foreground">
							<Checkbox
								checked={methodFilters.includes('none')}
								onCheckedChange={() => toggleMethodFilter('none')}
								aria-label="Toggle none method filter"
							/>
							None
						</label>
					</div>
					<div class="grid gap-2">
						<span class="text-[0.65rem] font-medium tracking-wide text-muted-foreground uppercase"
							>Field</span
						>
						<label class="flex items-center gap-2 text-xs text-muted-foreground">
							<Checkbox
								checked={fieldFilters.includes('ordered')}
								onCheckedChange={() => toggleFieldFilter('ordered')}
								aria-label="Toggle ordered field filter"
							/>
							Ordered
						</label>
						<label class="flex items-center gap-2 text-xs text-muted-foreground">
							<Checkbox
								checked={fieldFilters.includes('noise')}
								onCheckedChange={() => toggleFieldFilter('noise')}
								aria-label="Toggle noise field filter"
							/>
							Noise
						</label>
						<label class="flex items-center gap-2 text-xs text-muted-foreground">
							<Checkbox
								checked={fieldFilters.includes('kernel')}
								onCheckedChange={() => toggleFieldFilter('kernel')}
								aria-label="Toggle kernel field filter"
							/>
							Kernel
						</label>
						<label class="flex items-center gap-2 text-xs text-muted-foreground">
							<Checkbox
								checked={fieldFilters.includes('none')}
								onCheckedChange={() => toggleFieldFilter('none')}
								aria-label="Toggle none field filter"
							/>
							None
						</label>
					</div>
				</div>
			</SheetContent>
		</Sheet>

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

		<div class="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-2">
			<Label for="dither-placement" class="text-xs text-muted-foreground">Placement</Label>
			<Select bind:value={placement} type="single" disabled={!supportsPlacement}>
				<SelectTrigger id="dither-placement" class="w-full">{placementLabel}</SelectTrigger>
				<SelectContent>
					{#each PLACEMENT_MODES as opt (opt.id)}
						<SelectItem value={opt.id}>{opt.label}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
		</div>

		{#if placement === 'adaptive'}
			<div class="grid gap-2">
				<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
					<Label for="dither-placement-radius" class="text-xs text-muted-foreground">Radius</Label>
					<Slider
						type="single"
						bind:value={placementRadius}
						min={PLACEMENT_RADIUS_MIN}
						max={PLACEMENT_RADIUS_MAX}
						step={1}
						disabled={!supportsPlacement}
						aria-label="Adaptive placement radius"
					/>
					<div class="relative">
						<input
							id="dither-placement-radius"
							class="h-8 w-full border border-input bg-background px-2 pr-6 text-right font-mono text-xs tabular-nums"
							type="number"
							min={PLACEMENT_RADIUS_MIN}
							max={PLACEMENT_RADIUS_MAX}
							step="1"
							bind:value={placementRadius}
							disabled={!supportsPlacement}
						/>
						<span
							class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
							>px</span
						>
					</div>
				</div>
				<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
					<Label for="dither-placement-threshold" class="text-xs text-muted-foreground"
						>Threshold</Label
					>
					<Slider
						type="single"
						bind:value={placementThreshold}
						min={PLACEMENT_PERCENT_MIN}
						max={PLACEMENT_PERCENT_MAX}
						step={1}
						disabled={!supportsPlacement}
						aria-label="Adaptive placement threshold"
					/>
					<div class="relative">
						<input
							id="dither-placement-threshold"
							class="h-8 w-full border border-input bg-background px-2 pr-5 text-right font-mono text-xs tabular-nums"
							type="number"
							min={PLACEMENT_PERCENT_MIN}
							max={PLACEMENT_PERCENT_MAX}
							step="1"
							bind:value={placementThreshold}
							disabled={!supportsPlacement}
						/>
						<span
							class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
							>%</span
						>
					</div>
				</div>
				<div class="grid grid-cols-[5rem_minmax(0,1fr)_5.5rem] items-center gap-2">
					<Label for="dither-placement-softness" class="text-xs text-muted-foreground"
						>Softness</Label
					>
					<Slider
						type="single"
						bind:value={placementSoftness}
						min={PLACEMENT_PERCENT_MIN}
						max={PLACEMENT_PERCENT_MAX}
						step={1}
						disabled={!supportsPlacement}
						aria-label="Adaptive placement softness"
					/>
					<div class="relative">
						<input
							id="dither-placement-softness"
							class="h-8 w-full border border-input bg-background px-2 pr-5 text-right font-mono text-xs tabular-nums"
							type="number"
							min={PLACEMENT_PERCENT_MIN}
							max={PLACEMENT_PERCENT_MAX}
							step="1"
							bind:value={placementSoftness}
							disabled={!supportsPlacement}
						/>
						<span
							class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
							>%</span
						>
					</div>
				</div>
			</div>
		{/if}

		<Separator />

		<div class="grid gap-2">
			<div class="flex items-center justify-between gap-3">
				<Label for="dither-color-space" class="grid gap-0.5 text-left">
					<span>Dither in selected space</span>
					<span class="text-xs font-normal text-muted-foreground"
						>Perturb and diffuse error in the selected color space. Color matching always uses it.</span
					>
				</Label>
				<Switch
					id="dither-color-space"
					bind:checked={useColorSpace}
					disabled={!supportsColorSpaceDither}
				/>
			</div>

			<div class="flex items-center justify-between gap-3">
				<Label for="dither-serpentine" class="grid gap-0.5 text-left">
					<span>Serpentine scan</span>
					<span class="text-xs font-normal text-muted-foreground"
						>Reduces directional bias for error diffusion.</span
					>
				</Label>
				<Switch id="dither-serpentine" bind:checked={serpentine} disabled={!isErrorDiffusion} />
			</div>
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
