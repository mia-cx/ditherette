<script lang="ts">
	import AppBar from '$lib/components/skeletons/AppBar.svelte';
	import VariantNav from '$lib/components/skeletons/VariantNav.svelte';
	import ComparisonPreview from '$lib/components/skeletons/ComparisonPreview.svelte';
	import DitherPanel from '$lib/components/skeletons/DitherPanel.svelte';
	import ColorSpacePanel from '$lib/components/skeletons/ColorSpacePanel.svelte';
	import PalettePanel from '$lib/components/skeletons/PalettePanel.svelte';
	import OutputPanel from '$lib/components/skeletons/OutputPanel.svelte';
	import ExportStrip from '$lib/components/skeletons/ExportStrip.svelte';
	import { ResizablePaneGroup, ResizablePane, ResizableHandle } from '$lib/components/ui/resizable';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Separator } from '$lib/components/ui/separator';
	import { Accordion, AccordionItem, AccordionTrigger, AccordionContent } from '$lib/components/ui/accordion';
</script>

<svelte:head><title>v4 · Resizable — ditherette</title></svelte:head>

<div class="bg-background flex min-h-svh flex-col">
	<AppBar hasImage={false} />
	<VariantNav />

	<!-- Desktop: horizontal split with resize handle.
	     Mobile (< lg): stacked layout with collapsible accordion controls. -->
	<main class="flex flex-1 flex-col overflow-hidden lg:hidden">
		<div class="p-3">
			<ComparisonPreview hasImage={false} minHeightClass="min-h-[320px]" />
		</div>
		<div class="border-border flex-1 overflow-y-auto border-t">
			<Accordion type="multiple" class="px-3">
				<AccordionItem value="output">
					<AccordionTrigger>Output</AccordionTrigger>
					<AccordionContent><OutputPanel hasImage={false} hideHeading /></AccordionContent>
				</AccordionItem>
				<AccordionItem value="dither">
					<AccordionTrigger>Dither</AccordionTrigger>
					<AccordionContent><DitherPanel hideHeading /></AccordionContent>
				</AccordionItem>
				<AccordionItem value="color">
					<AccordionTrigger>Color space</AccordionTrigger>
					<AccordionContent><ColorSpacePanel hideHeading /></AccordionContent>
				</AccordionItem>
				<AccordionItem value="palette">
					<AccordionTrigger>Palette</AccordionTrigger>
					<AccordionContent><PalettePanel hideHeading /></AccordionContent>
				</AccordionItem>
			</Accordion>
		</div>
	</main>

	<main class="hidden flex-1 overflow-hidden lg:block">
		<ResizablePaneGroup direction="horizontal" class="h-full">
			<ResizablePane defaultSize={62} minSize={40}>
				<div class="flex h-full flex-col gap-3 overflow-hidden p-3">
					<ComparisonPreview hasImage={false} minHeightClass="flex-1 min-h-[400px]" />
				</div>
			</ResizablePane>

			<ResizableHandle withHandle />

			<ResizablePane defaultSize={38} minSize={26}>
				<ResizablePaneGroup direction="vertical" class="h-full">
					<ResizablePane defaultSize={55} minSize={20}>
						<ScrollArea class="h-full">
							<div class="flex flex-col gap-6 p-4">
								<OutputPanel hasImage={false} />
								<Separator />
								<DitherPanel />
								<Separator />
								<ColorSpacePanel />
							</div>
						</ScrollArea>
					</ResizablePane>
					<ResizableHandle withHandle />
					<ResizablePane defaultSize={45} minSize={20}>
						<div class="flex h-full flex-col p-4">
							<PalettePanel fillHeight />
						</div>
					</ResizablePane>
				</ResizablePaneGroup>
			</ResizablePane>
		</ResizablePaneGroup>
	</main>

	<ExportStrip variant="bar" hasImage={false} />
</div>
