<script lang="ts">
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
		AccordionContent,
	} from '$lib/components/ui/accordion';
	import { Badge } from '$lib/components/ui/badge';
	import {
		ResizablePaneGroup,
		ResizablePane,
		ResizableHandle,
	} from '$lib/components/ui/resizable';

	// Left-column accordion: all sections open by default so nothing is hidden
	// on first load. Users can collapse sections they're done with.
	let openSections = $state<string[]>(['output', 'dither', 'color']);
</script>

<svelte:head><title>ditherette</title></svelte:head>

<!--
	Layout strategy:
	- Mobile (< lg): natural page scroll. Preview, controls, palette, export stack.
	  Vertical resizing isn't meaningful when the page already scrolls freely.
	- Desktop (lg+): viewport-locked. Preview and controls live in a vertical
	  paneforge group with a draggable handle between them, so users can give
	  the preview as much (or as little) screen as they want. Each control
	  column still scrolls independently within the lower pane.
-->
<div class="bg-background flex min-h-svh flex-col lg:h-svh">
	<AppBar hasImage={false} />

	<!-- Mobile main: natural flow. -->
	<main class="flex flex-1 flex-col gap-4 lg:hidden">
		<ComparisonPreview
			hasImage={false}
			minHeightClass="min-h-[320px] md:min-h-[420px]"
		/>
		{@render controls('gap-4')}
	</main>

	<!-- Desktop main: vertical resizable panes. -->
	<main class="hidden flex-1 overflow-hidden lg:block">
		<ResizablePaneGroup direction="vertical" class="h-full">
			<ResizablePane defaultSize={56} minSize={25}>
				<ComparisonPreview hasImage={false} minHeightClass="h-full" />
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
		<ExportStrip variant="bar" hasImage={false} />
	</div>
</div>

<!--
	Two-column controls grid. Same children on mobile and desktop; only the
	wrapper sizing differs (mobile: free flow with gap-4; desktop: h-full with
	independently-scrolling columns inside the lower paneforge pane).
-->
{#snippet controls(extra: string)}
	<div
		class="grid lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] {extra}"
	>
		<!-- LEFT: collapsible Output / Dither / Color sections. -->
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

		<!-- RIGHT: palette card. Independently scrollable on desktop. -->
		<div class="flex min-h-0 flex-col lg:overflow-y-auto">
			<Card class="flex min-h-0 flex-1 flex-col py-3">
				<CardContent class="flex min-h-0 flex-1 flex-col">
					<PalettePanel fillHeight />
				</CardContent>
			</Card>
		</div>
	</div>
{/snippet}
