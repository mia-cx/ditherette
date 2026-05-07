<script lang="ts">
	import AppBar from '$lib/components/skeletons/AppBar.svelte';
	import VariantNav from '$lib/components/skeletons/VariantNav.svelte';
	import ComparisonPreview from '$lib/components/skeletons/ComparisonPreview.svelte';
	import DitherPanel from '$lib/components/skeletons/DitherPanel.svelte';
	import ColorSpacePanel from '$lib/components/skeletons/ColorSpacePanel.svelte';
	import PalettePanel from '$lib/components/skeletons/PalettePanel.svelte';
	import OutputPanel from '$lib/components/skeletons/OutputPanel.svelte';
	import ExportStrip from '$lib/components/skeletons/ExportStrip.svelte';
	import { Accordion, AccordionItem, AccordionTrigger, AccordionContent } from '$lib/components/ui/accordion';
	import { Badge } from '$lib/components/ui/badge';
</script>

<svelte:head><title>v6 · Accordion — ditherette</title></svelte:head>

<div class="bg-background flex min-h-svh flex-col">
	<AppBar hasImage={false} />
	<VariantNav />

	<main class="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-4 p-3 sm:p-4">
		<ComparisonPreview hasImage={false} minHeightClass="min-h-[320px] md:min-h-[440px]" />

		<!-- All control panels collapse cleanly. Default-open: Output + Palette so
		     the most-used controls are visible without taps. -->
		<Accordion
			type="multiple"
			value={['output', 'palette']}
			class="border-border bg-background border"
		>
			<AccordionItem value="output" class="border-b">
				<AccordionTrigger class="px-4">
					<span class="flex items-center gap-2">
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

			<AccordionItem value="dither" class="border-b">
				<AccordionTrigger class="px-4">
					<span class="flex items-center gap-2">
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

			<AccordionItem value="color" class="border-b">
				<AccordionTrigger class="px-4">
					<span class="flex items-center gap-2">
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

			<AccordionItem value="palette">
				<AccordionTrigger class="px-4">
					<span class="flex items-center gap-2">
						Palette
						<Badge variant="outline">Wplace · 64 / 64</Badge>
					</span>
				</AccordionTrigger>
				<AccordionContent>
					<div class="px-4 pb-4">
						<PalettePanel hideHeading />
					</div>
				</AccordionContent>
			</AccordionItem>
		</Accordion>
	</main>

	<div class="sticky bottom-0 z-20">
		<ExportStrip variant="bar" hasImage={false} />
	</div>
</div>
