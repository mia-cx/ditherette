<script lang="ts">
	import AppBar from '$lib/components/skeletons/AppBar.svelte';
	import VariantNav from '$lib/components/skeletons/VariantNav.svelte';
	import ComparisonPreview from '$lib/components/skeletons/ComparisonPreview.svelte';
	import DitherPanel from '$lib/components/skeletons/DitherPanel.svelte';
	import ColorSpacePanel from '$lib/components/skeletons/ColorSpacePanel.svelte';
	import PalettePanel from '$lib/components/skeletons/PalettePanel.svelte';
	import OutputPanel from '$lib/components/skeletons/OutputPanel.svelte';
	import ExportStrip from '$lib/components/skeletons/ExportStrip.svelte';
	import { Drawer, DrawerContent, DrawerHeader, DrawerTitle, DrawerTrigger } from '$lib/components/ui/drawer';
	import { Button } from '$lib/components/ui/button';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Tabs, TabsList, TabsTrigger, TabsContent } from '$lib/components/ui/tabs';
	import { Badge } from '$lib/components/ui/badge';
	import RulerIcon from 'phosphor-svelte/lib/Ruler';
	import PaintBrushIcon from 'phosphor-svelte/lib/PaintBrushHousehold';
	import EyedropperIcon from 'phosphor-svelte/lib/Eyedropper';
	import PaletteIcon from 'phosphor-svelte/lib/Palette';
	import DownloadIcon from 'phosphor-svelte/lib/DownloadSimple';

	type DrawerKey = 'output' | 'dither' | 'color' | 'palette';
	let openDrawer = $state<DrawerKey | null>(null);

	const TRIGGERS: { key: DrawerKey; label: string; icon: typeof RulerIcon }[] = [
		{ key: 'output', label: 'Output', icon: RulerIcon },
		{ key: 'dither', label: 'Dither', icon: PaintBrushIcon },
		{ key: 'color', label: 'Color', icon: EyedropperIcon },
		{ key: 'palette', label: 'Palette', icon: PaletteIcon },
	];
</script>

<svelte:head><title>v5 · Drawer — ditherette</title></svelte:head>

<div class="bg-background flex h-svh flex-col overflow-hidden">
	<AppBar hasImage={false} dense />
	<VariantNav />

	<!-- Hero takes full remaining space; controls live in drawers below. -->
	<main class="flex flex-1 flex-col overflow-hidden p-2 sm:p-3">
		<ComparisonPreview hasImage={false} minHeightClass="flex-1 min-h-[280px]" />
	</main>

	<!-- Bottom action rail: tappable triggers + primary download. -->
	<nav
		class="border-border bg-background/95 grid grid-cols-5 border-t backdrop-blur"
		aria-label="Controls"
	>
		{#each TRIGGERS as t (t.key)}
			<Drawer
				open={openDrawer === t.key}
				onOpenChange={(o) => (openDrawer = o ? t.key : null)}
			>
				<DrawerTrigger
					class="hover:bg-muted active:bg-muted flex h-14 w-full flex-col items-center justify-center gap-0.5"
				>
					<t.icon class="size-5" />
					<span class="text-[10px] font-medium uppercase tracking-wide">{t.label}</span>
				</DrawerTrigger>
				<DrawerContent class="max-h-[88svh]">
					<DrawerHeader class="text-left">
						<DrawerTitle class="capitalize">{t.label}</DrawerTitle>
					</DrawerHeader>
					<ScrollArea class="px-4 pb-6">
						{#if t.key === 'output'}
							<OutputPanel hasImage={false} hideHeading />
						{:else if t.key === 'dither'}
							<DitherPanel hideHeading />
						{:else if t.key === 'color'}
							<ColorSpacePanel hideHeading />
						{:else}
							<PalettePanel hideHeading />
						{/if}
					</ScrollArea>
				</DrawerContent>
			</Drawer>
		{/each}

		<Drawer>
			<DrawerTrigger
				class="bg-primary text-primary-foreground hover:bg-primary/90 flex h-14 w-full flex-col items-center justify-center gap-0.5"
				aria-label="Export"
			>
				<DownloadIcon class="size-5" />
				<span class="text-[10px] font-medium uppercase tracking-wide">Export</span>
			</DrawerTrigger>
			<DrawerContent>
				<DrawerHeader class="text-left">
					<DrawerTitle class="flex items-center gap-2">
						Export <Badge variant="outline" class="font-mono">PNG</Badge>
					</DrawerTitle>
				</DrawerHeader>
				<div class="px-4 pb-6">
					<ExportStrip variant="card" hasImage={false} />
				</div>
			</DrawerContent>
		</Drawer>
	</nav>
</div>
