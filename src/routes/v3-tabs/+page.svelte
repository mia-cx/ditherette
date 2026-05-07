<script lang="ts">
	import AppBar from '$lib/components/skeletons/AppBar.svelte';
	import VariantNav from '$lib/components/skeletons/VariantNav.svelte';
	import ComparisonPreview from '$lib/components/skeletons/ComparisonPreview.svelte';
	import DitherPanel from '$lib/components/skeletons/DitherPanel.svelte';
	import ColorSpacePanel from '$lib/components/skeletons/ColorSpacePanel.svelte';
	import PalettePanel from '$lib/components/skeletons/PalettePanel.svelte';
	import OutputPanel from '$lib/components/skeletons/OutputPanel.svelte';
	import ExportStrip from '$lib/components/skeletons/ExportStrip.svelte';
	import { Tabs, TabsList, TabsTrigger, TabsContent } from '$lib/components/ui/tabs';
	import { Card, CardContent } from '$lib/components/ui/card';
	import PaintBrushIcon from 'phosphor-svelte/lib/PaintBrushHousehold';
	import PaletteIcon from 'phosphor-svelte/lib/Palette';
	import EyedropperIcon from 'phosphor-svelte/lib/Eyedropper';
	import RulerIcon from 'phosphor-svelte/lib/Ruler';

	let tab = $state('output');
</script>

<svelte:head><title>v3 · Tabs — ditherette</title></svelte:head>

<div class="bg-background flex min-h-svh flex-col">
	<AppBar hasImage={false} />
	<VariantNav />

	<main class="mx-auto flex w-full max-w-7xl flex-1 flex-col gap-4 p-3 sm:p-4">
		<ComparisonPreview hasImage={false} minHeightClass="min-h-[340px] md:min-h-[460px]" />

		<Card class="overflow-hidden">
			<Tabs bind:value={tab} class="w-full">
				<TabsList class="h-auto w-full justify-start overflow-x-auto rounded-none border-b">
					<TabsTrigger value="output">
						<RulerIcon />
						<span>Output</span>
					</TabsTrigger>
					<TabsTrigger value="dither">
						<PaintBrushIcon />
						<span>Dither</span>
					</TabsTrigger>
					<TabsTrigger value="color">
						<EyedropperIcon />
						<span>Color space</span>
					</TabsTrigger>
					<TabsTrigger value="palette">
						<PaletteIcon />
						<span>Palette</span>
					</TabsTrigger>
				</TabsList>

				<TabsContent value="output">
					<CardContent class="grid gap-6 p-4 md:grid-cols-2">
						<OutputPanel hasImage={false} hideHeading />
					</CardContent>
				</TabsContent>
				<TabsContent value="dither">
					<CardContent class="p-4">
						<DitherPanel hideHeading />
					</CardContent>
				</TabsContent>
				<TabsContent value="color">
					<CardContent class="p-4">
						<ColorSpacePanel hideHeading />
					</CardContent>
				</TabsContent>
				<TabsContent value="palette">
					<CardContent class="p-4">
						<PalettePanel hideHeading />
					</CardContent>
				</TabsContent>
			</Tabs>
		</Card>
	</main>

	<div class="sticky bottom-0 z-20">
		<ExportStrip variant="bar" hasImage={false} />
	</div>
</div>
