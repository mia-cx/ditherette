<script lang="ts">
	import { onMount } from 'svelte';
	import AppBar from './components/AppBar.svelte';
	import ComparisonPreview from './components/ComparisonPreview.svelte';
	import DitherPanel from './components/DitherPanel.svelte';
	import ColorSpacePanel from './components/ColorSpacePanel.svelte';
	import PalettePanel from './components/PalettePanel.svelte';
	import OutputPanel from './components/OutputPanel.svelte';
	import ExportStrip from './components/ExportStrip.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
	import {
		Accordion,
		AccordionItem,
		AccordionTrigger,
		AccordionContent
	} from '$lib/components/ui/accordion';
	import { Badge } from '$lib/components/ui/badge';
	import { ResizablePaneGroup, ResizablePane, ResizableHandle } from '$lib/components/ui/resizable';
	import {
		colorSpace,
		ditherSettings,
		hasImage,
		outputSettings,
		processedImage
	} from '$lib/stores/app';
	import { startAutoProcessing } from '$lib/processing/client';
	import {
		clearAllImageData,
		hasPersistedSourceImage,
		restorePersistedImages,
		setSourceFile
	} from '$lib/processing/source';
	import { COLOR_SPACES, DITHER_ALGORITHMS, RESIZE_MODES } from './components/sample-data';

	let openSections = $state<string[]>(['output', 'dither', 'color']);
	let fileInput = $state<HTMLInputElement>();
	let uploadError = $state<string>();

	const outputBadge = $derived(
		`${$processedImage?.width ?? $outputSettings.width}×${$processedImage?.height ?? $outputSettings.height} · ${RESIZE_MODES.find((mode) => mode.id === $outputSettings.resize)?.label ?? 'Resize'}`
	);
	const ditherBadge = $derived(
		DITHER_ALGORITHMS.find((mode) => mode.id === $ditherSettings.algorithm)?.label ?? 'Off'
	);
	const colorBadge = $derived(
		COLOR_SPACES.find((mode) => mode.id === $colorSpace)?.label ?? 'OKLab'
	);

	onMount(() => {
		const stop = startAutoProcessing();
		void restorePreviousSession();
		return stop;
	});

	async function restorePreviousSession() {
		try {
			if (!(await hasPersistedSourceImage())) return;
			const restore = confirm(
				'Ditherette has a locally saved image from a previous session. Restore it? Choose Cancel to delete the saved image from this browser.'
			);
			if (!restore) {
				await clearAllImageData();
				return;
			}
			await restorePersistedImages();
		} catch (error) {
			uploadError = error instanceof Error ? error.message : 'Could not restore saved image.';
		}
	}

	function chooseImage() {
		fileInput?.click();
	}

	async function loadImageFile(file: File) {
		if ($hasImage && !confirm('Replace the current image with this file?')) return;
		uploadError = undefined;
		try {
			await setSourceFile(file);
		} catch (error) {
			uploadError = error instanceof Error ? error.message : 'Could not read that image.';
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
		onClear={() => void clearAllImageData()}
	/>

	{#if uploadError}
		<p class="border-b border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
			{uploadError}
		</p>
	{/if}

	<main class="flex flex-1 flex-col gap-4 lg:hidden">
		<ComparisonPreview
			defaultMode="ab-reveal"
			hasImage={$hasImage}
			minHeightClass="min-h-[320px] md:min-h-[420px]"
			onChooseImage={chooseImage}
			onSelectFile={(file) => void loadImageFile(file)}
		/>
		{@render controls('gap-4')}
	</main>

	<main class="hidden flex-1 overflow-hidden lg:block">
		<ResizablePaneGroup direction="vertical" class="h-full">
			<ResizablePane defaultSize={56} minSize={25}>
				<ComparisonPreview
					hasImage={$hasImage}
					minHeightClass="h-full"
					onChooseImage={chooseImage}
					onSelectFile={(file) => void loadImageFile(file)}
				/>
			</ResizablePane>
			<ResizableHandle withHandle />
			<ResizablePane defaultSize={44} minSize={25}>
				<div class="h-full overflow-hidden p-0 pt-3">
					{@render controls('gap-3 h-full overflow-hidden')}
				</div>
			</ResizablePane>
		</ResizablePaneGroup>
	</main>

	<div class="sticky bottom-0 z-20 lg:static">
		<ExportStrip variant="bar" hasImage={$hasImage} />
	</div>
</div>

{#snippet controls(extra: string)}
	<div class="grid lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] {extra}">
		<div class="flex min-h-0 flex-col lg:overflow-y-auto">
			<Accordion type="multiple" bind:value={openSections} class="border border-border bg-card">
				<AccordionItem value="output">
					<AccordionTrigger class="px-4">
						<span class="flex items-center gap-2 text-sm">
							Output
							<Badge variant="outline" class="font-mono">{outputBadge}</Badge>
						</span>
					</AccordionTrigger>
					<AccordionContent>
						<div class="px-4 pb-4">
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
						<div class="px-4 pb-4">
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
						<div class="px-4 pb-4">
							<ColorSpacePanel hideHeading />
						</div>
					</AccordionContent>
				</AccordionItem>
			</Accordion>
		</div>

		<div class="flex min-h-0 flex-col lg:overflow-y-auto">
			<Card class="flex min-h-0 flex-1 flex-col py-3">
				<CardContent class="flex min-h-0 flex-1 flex-col">
					<PalettePanel fillHeight />
				</CardContent>
			</Card>
		</div>
	</div>
{/snippet}
