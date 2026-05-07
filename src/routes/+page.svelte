<script lang="ts">
	import AppBar from '$lib/components/skeletons/AppBar.svelte';
	import ComparisonPreview from '$lib/components/skeletons/ComparisonPreview.svelte';
	import DitherPanel from '$lib/components/skeletons/DitherPanel.svelte';
	import ColorSpacePanel from '$lib/components/skeletons/ColorSpacePanel.svelte';
	import PalettePanel from '$lib/components/skeletons/PalettePanel.svelte';
	import OutputPanel from '$lib/components/skeletons/OutputPanel.svelte';
	import ExportStrip from '$lib/components/skeletons/ExportStrip.svelte';
	import { Card, CardContent } from '$lib/components/ui/card';
	import {
		Accordion,
		AccordionItem,
		AccordionTrigger,
		AccordionContent,
	} from '$lib/components/ui/accordion';
	import { Badge } from '$lib/components/ui/badge';

	// Left-column accordion: all sections open by default so nothing is hidden
	// on first load. Users can collapse sections they're done with.
	let openSections = $state<string[]>(['output', 'dither', 'color']);
</script>

<svelte:head><title>ditherette</title></svelte:head>

<!--
	Layout strategy:
	- Mobile (< lg): natural page scroll. Preview, controls, palette, export stack.
	- Desktop (lg+): viewport-locked. Hero preview takes a bounded slice at the
	  top of the main area; the two control columns below split the remaining
	  height and scroll independently. Export strip sticks to the bottom.
-->
<div class="bg-background flex min-h-svh flex-col lg:h-svh">
	<AppBar hasImage={false} />

	<main
		class="flex flex-1 flex-col gap-4 p-3 sm:p-4 lg:min-h-0 lg:gap-3 lg:overflow-hidden"
	>
		<!-- Hero preview: NOT inside a card, per spec. -->
		<ComparisonPreview
			hasImage={false}
			minHeightClass="min-h-[260px] md:min-h-[320px] lg:min-h-[360px] lg:max-h-[52svh]"
		/>

		<!-- Two-column controls on lg+, stacked on mobile.
		     Mobile order: Output → Dither → Color Space → Palette. -->
		<div
			class="grid gap-4 lg:min-h-0 lg:flex-1 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.15fr)] lg:gap-3 lg:overflow-hidden"
		>
			<!-- LEFT COLUMN: collapsible Output / Dither / Color sections.
			     Independently scrollable on desktop. -->
			<div class="flex min-h-0 flex-col lg:overflow-y-auto">
				<Accordion
					type="multiple"
					bind:value={openSections}
					class="border-border bg-card border"
				>
					<AccordionItem value="output">
						<AccordionTrigger class="px-4">
							<span class="flex items-center gap-2 text-sm">
								Output
								<Badge variant="outline" class="font-mono">512×512 · Lanczos3</Badge>
							</span>
						</AccordionTrigger>
						<AccordionContent>
							<div class="px-4 pb-4">
								<OutputPanel hasImage={false} hideHeading />
							</div>
						</AccordionContent>
					</AccordionItem>

					<AccordionItem value="dither">
						<AccordionTrigger class="px-4">
							<span class="flex items-center gap-2 text-sm">
								Dither
								<Badge variant="secondary">Off</Badge>
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
								<Badge variant="outline">OKLab</Badge>
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

			<!-- RIGHT COLUMN: palette card. Independently scrollable on desktop;
			     the table inside has its own ScrollArea so the card chrome stays. -->
			<div class="flex min-h-0 flex-col lg:overflow-y-auto">
				<Card class="flex min-h-0 flex-1 flex-col">
					<CardContent class="flex min-h-0 flex-1 flex-col p-4">
						<PalettePanel fillHeight />
					</CardContent>
				</Card>
			</div>
		</div>
	</main>

	<div class="sticky bottom-0 z-20 lg:static">
		<ExportStrip variant="bar" hasImage={false} />
	</div>
</div>
