<script lang="ts">
	import AppBar from '$lib/components/skeletons/AppBar.svelte';
	import VariantNav from '$lib/components/skeletons/VariantNav.svelte';
	import ComparisonPreview from '$lib/components/skeletons/ComparisonPreview.svelte';
	import DitherPanel from '$lib/components/skeletons/DitherPanel.svelte';
	import ColorSpacePanel from '$lib/components/skeletons/ColorSpacePanel.svelte';
	import PalettePanel from '$lib/components/skeletons/PalettePanel.svelte';
	import OutputPanel from '$lib/components/skeletons/OutputPanel.svelte';
	import ExportStrip from '$lib/components/skeletons/ExportStrip.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
</script>

<svelte:head><title>v1 · Stack — ditherette</title></svelte:head>

<div class="bg-background flex min-h-svh flex-col">
	<AppBar hasImage={false} />
	<VariantNav />

	<main class="flex flex-1 flex-col gap-4 p-3 sm:p-4">
		<!-- Hero preview: NOT inside a card, per spec. -->
		<ComparisonPreview hasImage={false} minHeightClass="min-h-[360px] md:min-h-[480px]" />

		<!-- Two-column controls on md+, stacked on mobile.
		     Mobile order: Output → Dither → Color Space → Palette. -->
		<div class="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
			<div class="flex flex-col gap-4">
				<Card>
					<CardContent class="p-4">
						<OutputPanel hasImage={false} />
					</CardContent>
				</Card>
				<Card>
					<CardContent class="p-4">
						<DitherPanel />
					</CardContent>
				</Card>
				<Card>
					<CardContent class="p-4">
						<ColorSpacePanel />
					</CardContent>
				</Card>
			</div>
			<div class="flex flex-col gap-4">
				<Card class="lg:sticky lg:top-20 lg:max-h-[calc(100svh-6rem)]">
					<CardContent class="flex h-full flex-col p-4">
						<PalettePanel fillHeight />
					</CardContent>
				</Card>
			</div>
		</div>
	</main>

	<div class="sticky bottom-0 z-20">
		<ExportStrip variant="bar" hasImage={false} />
	</div>
</div>
