<script lang="ts">
	import AppBar from '$lib/components/skeletons/AppBar.svelte';
	import VariantNav from '$lib/components/skeletons/VariantNav.svelte';
	import ComparisonPreview from '$lib/components/skeletons/ComparisonPreview.svelte';
	import DitherPanel from '$lib/components/skeletons/DitherPanel.svelte';
	import ColorSpacePanel from '$lib/components/skeletons/ColorSpacePanel.svelte';
	import PalettePanel from '$lib/components/skeletons/PalettePanel.svelte';
	import OutputPanel from '$lib/components/skeletons/OutputPanel.svelte';
	import ExportStrip from '$lib/components/skeletons/ExportStrip.svelte';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Sheet, SheetContent, SheetTrigger, SheetHeader, SheetTitle } from '$lib/components/ui/sheet';
	import { Button } from '$lib/components/ui/button';
	import { Separator } from '$lib/components/ui/separator';
	import SlidersIcon from 'phosphor-svelte/lib/SlidersHorizontal';
</script>

<svelte:head><title>v2 · Sidebar — ditherette</title></svelte:head>

<div class="bg-background flex min-h-svh flex-col">
	<AppBar hasImage={false}>
		{#snippet extras()}
			<!-- Mobile-only sheet trigger; desktop has a persistent rail. -->
			<Sheet>
				<SheetTrigger class="lg:hidden">
					{#snippet child({ props })}
						<Button {...props} size="icon-sm" variant="outline" aria-label="Open controls">
							<SlidersIcon />
						</Button>
					{/snippet}
				</SheetTrigger>
				<SheetContent side="right" class="w-[88vw] max-w-md p-0">
					<SheetHeader class="border-border border-b">
						<SheetTitle>Controls</SheetTitle>
					</SheetHeader>
					<ScrollArea class="h-[calc(100svh-3.5rem)]">
						<div class="flex flex-col gap-6 p-4">
							<OutputPanel hasImage={false} />
							<Separator />
							<DitherPanel />
							<Separator />
							<ColorSpacePanel />
							<Separator />
							<PalettePanel />
						</div>
					</ScrollArea>
				</SheetContent>
			</Sheet>
		{/snippet}
	</AppBar>
	<VariantNav />

	<div class="flex flex-1 overflow-hidden">
		<!-- Persistent left rail (desktop only). -->
		<aside
			class="border-border hidden w-[320px] shrink-0 flex-col border-r lg:flex xl:w-[360px]"
			aria-label="Controls"
		>
			<ScrollArea class="flex-1">
				<div class="flex flex-col gap-6 p-4">
					<OutputPanel hasImage={false} />
					<Separator />
					<DitherPanel />
					<Separator />
					<ColorSpacePanel />
				</div>
			</ScrollArea>
		</aside>

		<!-- Main work area: preview + palette below it. -->
		<main class="flex flex-1 flex-col gap-4 overflow-y-auto p-3 sm:p-4">
			<ComparisonPreview hasImage={false} minHeightClass="min-h-[360px] md:min-h-[520px]" />
			<section class="border-border bg-background border p-4">
				<PalettePanel />
			</section>
		</main>

		<!-- Optional desktop palette rail on very wide screens. -->
		<aside
			class="border-border hidden w-[340px] shrink-0 flex-col border-l 2xl:flex"
			aria-label="Palette"
		>
			<ScrollArea class="flex-1">
				<div class="p-4">
					<PalettePanel fillHeight />
				</div>
			</ScrollArea>
		</aside>
	</div>

	<ExportStrip variant="bar" hasImage={false} />
</div>
