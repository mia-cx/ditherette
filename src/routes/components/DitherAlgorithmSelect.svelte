<script lang="ts">
	import { Accordion, AccordionItem } from '$lib/components/ui/accordion';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { Sheet, SheetContent, SheetHeader, SheetTitle } from '$lib/components/ui/sheet';
	import type { ColorSpaceId, DitherId, EnabledPaletteColor } from '$lib/processing/types';
	import { colorSpace, selectedPalette, uiSettings } from '$lib/stores/app';
	import {
		DITHER_ALGORITHMS,
		type DitherField,
		type DitherMethod,
		type DitherOption
	} from './dither-options';
	import InlineMath from './InlineMath.svelte';
	import DitherFilterGroups from './DitherFilterGroups.svelte';

	type DitherPreviewParams = {
		mode: DitherId;
		randomSeed: number;
		previewStrength: number;
		serpentineScan: boolean;
		palette: readonly EnabledPaletteColor[];
		colorSpaceMode: ColorSpaceId;
		useColorSpace: boolean;
	};

	type DitherPreviewAction = (
		target: HTMLCanvasElement,
		params: DitherPreviewParams
	) => { update(next: DitherPreviewParams): void } | void;

	type Props = {
		algorithm: DitherId;
		seed: number;
		strength: number;
		serpentine: boolean;
		useColorSpace: boolean;
		ditherPreview: DitherPreviewAction;
	};

	let {
		algorithm = $bindable(),
		seed,
		strength,
		serpentine,
		useColorSpace,
		ditherPreview
	}: Props = $props();

	let algorithmSearch = $state('');
	let methodFilters = $state<DitherMethod[]>(['none', 'threshold', 'error-diffusion']);
	let fieldFilters = $state<DitherField[]>(['none', 'ordered', 'noise', 'kernel']);
	let filterSheetOpen = $state(false);
	let desktopFilterSections = $state<string[]>(
		uiSettings.get().desktopDitherFiltersOpen ? ['filters'] : []
	);

	const current = $derived(DITHER_ALGORITHMS.find((option) => option.id === algorithm));
	const triggerLabel = $derived(current?.label ?? 'Select algorithm');
	const filteredAlgorithms = $derived(
		DITHER_ALGORITHMS.filter((option) => {
			const query = algorithmSearch.trim().toLowerCase();
			const matchesQuery =
				query.length === 0 ||
				[option.label, option.short, option.sku, option.method, option.field]
					.join(' ')
					.toLowerCase()
					.includes(query);
			return (
				matchesQuery && methodFilters.includes(option.method) && fieldFilters.includes(option.field)
			);
		})
	);

	$effect(() => {
		uiSettings.set({
			...uiSettings.get(),
			desktopDitherFiltersOpen: desktopFilterSections.includes('filters')
		});
	});

	function previewParams(option: DitherOption): DitherPreviewParams {
		return {
			mode: option.id,
			randomSeed: seed,
			previewStrength: strength,
			serpentineScan: serpentine,
			palette: $selectedPalette,
			colorSpaceMode: $colorSpace,
			useColorSpace
		};
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
</script>

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
						use:ditherPreview={previewParams(current)}
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
		interactOutsideBehavior="defer-otherwise-ignore"
		preventScroll={false}
		viewportFlex
		class="flex h-[min(38rem,var(--bits-select-content-available-height))] w-[calc(100vw-2rem)] max-w-[calc(100vw-2rem)] flex-col overflow-hidden p-1 md:w-auto md:max-w-none"
	>
		<div class="sticky top-0 z-10 border-b border-border bg-popover p-2">
			<div class="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] gap-2">
				<input
					class="h-8 min-w-0 border border-input bg-background px-2 text-xs outline-none focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring/50"
					placeholder="Search algorithms…"
					aria-label="Search algorithms"
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
						(desktopFilterSections = desktopFilterSections.includes('filters') ? [] : ['filters'])}
					>Filters</Button
				>
			</div>
		</div>
		<div class="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_auto] gap-2 overflow-hidden">
			<div class="min-w-0 overflow-y-auto">
				{#each filteredAlgorithms as option (option.id)}
					<SelectItem
						value={option.id}
						label={option.label}
						class="min-w-0 items-center py-3 pr-8 pl-3"
					>
						<span class="grid min-w-0 flex-1 gap-1.5">
							<span class="flex items-start gap-3">
								<canvas
									use:ditherPreview={previewParams(option)}
									class="size-20 shrink-0 bg-muted [image-rendering:pixelated] sm:size-24"
									aria-hidden="true"
								></canvas>
								<span class="grid min-w-0 flex-1 content-start gap-1">
									<span class="flex min-w-0 flex-wrap items-start gap-1.5">
										<span class="truncate text-sm font-medium text-foreground">{option.label}</span>
										<Badge variant="secondary">{methodLabel(option.method)}</Badge>
										<Badge variant="outline">{fieldLabel(option.field)}</Badge>
									</span>
									<span class="text-xs leading-relaxed whitespace-normal text-muted-foreground"
										>{option.short}</span
									>
									<span class="rounded-sm border border-border bg-muted/40 px-2 py-1">
										<InlineMath expression={option.latex} />
									</span>
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
				class="hidden h-full shrink-0 overflow-hidden border-l border-border md:flex"
			>
				<AccordionItem value="filters" class="h-full border-b-0">
					<div
						class={desktopFilterSections.includes('filters')
							? 'h-full w-40 overflow-hidden transition-[width] duration-200 ease-in-out'
							: 'h-full w-0 overflow-hidden transition-[width] duration-200 ease-in-out'}
					>
						<div class="grid w-40 gap-3 p-2 pb-2.5">
							<DitherFilterGroups
								{methodFilters}
								{fieldFilters}
								onToggleMethod={toggleMethodFilter}
								onToggleField={toggleFieldFilter}
								compact
							/>
						</div>
					</div>
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
			<DitherFilterGroups
				{methodFilters}
				{fieldFilters}
				onToggleMethod={toggleMethodFilter}
				onToggleField={toggleFieldFilter}
			/>
		</div>
	</SheetContent>
</Sheet>
