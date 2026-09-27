<script lang="ts">
	import { onMount } from 'svelte';
	import AppBar from './components/AppBar.svelte';
	import ComparisonPreview from './components/ComparisonPreview.svelte';
	import DitherPanel from './components/DitherPanel.svelte';
	import ColorSpacePanel from './components/ColorSpacePanel.svelte';
	import PalettePanel from './components/PalettePanel.svelte';
	import OutputPanel from './components/OutputPanel.svelte';
	import ExportStrip from './components/ExportStrip.svelte';
	import PerformanceDebugPopover from './components/PerformanceDebugPopover.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
	import {
		Accordion,
		AccordionItem,
		AccordionTrigger,
		AccordionContent
	} from '$lib/components/ui/accordion';
	import { Badge } from '$lib/components/ui/badge';
	import { MediaQuery } from 'svelte/reactivity';
	import {
		colorSpace,
		ditherSettings,
		hasImage,
		outputSettings,
		processedImage,
		uiSettings
	} from '$lib/stores/app';
	import { effectLayers } from '$lib/stores/effects';
	import PipelinePanel from './components/effects/PipelinePanel.svelte';
	import Studio from './components/studio/Studio.svelte';
	import WindowsMenu from './components/studio/WindowsMenu.svelte';
	import { startAutoProcessing } from '$lib/processing/client';
	import {
		clearAllImageData,
		isSourceSuperseded,
		restorePersistedImages,
		setSourceFile
	} from '$lib/processing/source';
	import { COLOR_SPACES } from './components/color-space-options';
	import { DITHER_ALGORITHMS } from './components/dither-options';
	import { RESIZE_MODES } from './components/output-options';

	let openSections = $state<string[]>(
		uiSettings.get().controlAccordionSections ?? ['dimensions', 'dither', 'color']
	);
	let fileInput = $state<HTMLInputElement>();
	let uploadError = $state<string>();

	// The studio needs room for docked windows; smaller screens stack the same controls.
	const studio = new MediaQuery('min-width: 1024px', true);

	const outputBadge = $derived(
		`${$processedImage?.width ?? $outputSettings.width}×${$processedImage?.height ?? $outputSettings.height} · ${RESIZE_MODES.find((mode) => mode.id === $outputSettings.resize)?.label ?? 'Resize'}`
	);
	const ditherBadge = $derived(
		DITHER_ALGORITHMS.find((mode) => mode.id === $ditherSettings.algorithm)?.label ?? 'Off'
	);
	const colorBadge = $derived(
		COLOR_SPACES.find((mode) => mode.id === $colorSpace)?.label ?? 'OKLab'
	);

	$effect(() => {
		uiSettings.set({ ...uiSettings.get(), controlAccordionSections: openSections });
	});

	onMount(() => {
		const stop = startAutoProcessing();
		void restorePersistedImages().catch((error) => {
			if (isSourceSuperseded(error)) return;
			uploadError = error instanceof Error ? error.message : 'Could not restore saved image.';
		});
		return stop;
	});

	function chooseImage() {
		fileInput?.click();
	}

	async function loadImageFile(file: File) {
		if ($hasImage && !confirm('Replace the current image with this file?')) return;
		uploadError = undefined;
		try {
			await setSourceFile(file);
		} catch (error) {
			if (isSourceSuperseded(error)) return;
			uploadError = error instanceof Error ? error.message : 'Could not read that image.';
		}
	}

	async function clearImageData() {
		uploadError = undefined;
		try {
			await clearAllImageData();
		} catch (error) {
			uploadError = error instanceof Error ? error.message : 'Could not clear saved image data.';
		}
	}

	async function onFileChange(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		try {
			await loadImageFile(file);
		} finally {
			input.value = '';
		}
	}
</script>

<svelte:head><title>ditherette</title></svelte:head>

<input
	bind:this={fileInput}
	class="sr-only"
	type="file"
	accept="image/png,image/jpeg,image/webp,image/gif"
	onchange={onFileChange}
/>

<div class="flex min-h-svh flex-col bg-background lg:h-svh">
	<AppBar
		hasImage={$hasImage}
		onChooseImage={chooseImage}
		onClear={clearImageData}
		extras={appBarExtras}
	/>

	{#if uploadError}
		<p class="border-b border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
			{uploadError}
		</p>
	{/if}

	<main class="flex min-h-0 flex-1 flex-col">
		{#if studio.current}
			<Studio onChooseImage={chooseImage} onSelectFile={(file) => void loadImageFile(file)} />
		{:else}
			<div class="flex flex-1 flex-col gap-4">
				<ComparisonPreview
					defaultMode="ab-reveal"
					hasImage={$hasImage}
					minHeightClass="min-h-[320px] md:min-h-[420px]"
					onChooseImage={chooseImage}
					onSelectFile={(file) => void loadImageFile(file)}
				/>
				{@render stackedControls()}
			</div>
		{/if}
	</main>

	<div class="sticky bottom-0 z-20 lg:static">
		<ExportStrip variant="bar" hasImage={$hasImage} />
	</div>
</div>

{#snippet appBarExtras()}
	{#if studio.current}
		<WindowsMenu />
	{/if}
	<PerformanceDebugPopover />
{/snippet}

{#snippet stackedControls()}
	<div class="grid gap-4">
		<Accordion type="multiple" bind:value={openSections} class="border border-border bg-card">
			<AccordionItem value="effects">
				<AccordionTrigger class="px-4">
					<span class="flex items-center gap-2 text-sm">
						Effects
						<Badge variant="outline" class="font-mono">{$effectLayers.length}</Badge>
					</span>
				</AccordionTrigger>
				<AccordionContent>
					<div class="p-4">
						<PipelinePanel />
					</div>
				</AccordionContent>
			</AccordionItem>

			<AccordionItem value="dimensions">
				<AccordionTrigger class="px-4">
					<span class="flex items-center gap-2 text-sm">
						Dimensions
						<Badge variant="outline" class="font-mono">{outputBadge}</Badge>
					</span>
				</AccordionTrigger>
				<AccordionContent>
					<div class="p-4">
						<OutputPanel hasImage={$hasImage} hideHeading />
					</div>
				</AccordionContent>
			</AccordionItem>

			<AccordionItem value="dither">
				<AccordionTrigger class="px-4">
					<span class="flex items-center gap-2 text-sm">
						Dither
						<Badge variant="secondary">{ditherBadge}</Badge>
					</span>
				</AccordionTrigger>
				<AccordionContent>
					<div class="p-4">
						<DitherPanel hideHeading />
					</div>
				</AccordionContent>
			</AccordionItem>

			<AccordionItem value="color">
				<AccordionTrigger class="px-4">
					<span class="flex items-center gap-2 text-sm">
						Color space
						<Badge variant="outline">{colorBadge}</Badge>
					</span>
				</AccordionTrigger>
				<AccordionContent>
					<div class="p-4">
						<ColorSpacePanel hideHeading />
					</div>
				</AccordionContent>
			</AccordionItem>
		</Accordion>

		<Card class="py-3">
			<CardContent>
				<PalettePanel />
			</CardContent>
		</Card>
	</div>
{/snippet}
