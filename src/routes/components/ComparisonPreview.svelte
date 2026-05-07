<script lang="ts">
	import { Tabs, TabsList, TabsTrigger } from '$lib/components/ui/tabs';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Slider } from '$lib/components/ui/slider';
	import { Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '$lib/components/ui/empty';
	import CropIcon from 'phosphor-svelte/lib/Crop';
	import ArrowsOutIcon from 'phosphor-svelte/lib/ArrowsOut';
	import MagnifyingGlassIcon from 'phosphor-svelte/lib/MagnifyingGlass';
	import ImageIcon from 'phosphor-svelte/lib/ImageSquare';
	import UploadIcon from 'phosphor-svelte/lib/UploadSimple';

	type PreviewMode = 'side-by-side' | 'ab-reveal';

	type Props = {
		// Initial preview mode. Per spec, mobile/narrow screens default to A/B
		// reveal and desktop/wide screens default to side-by-side. Each layout
		// passes the appropriate default since they render separate instances.
		defaultMode?: PreviewMode;
		hasImage?: boolean;
		minHeightClass?: string;
	};

	let {
		defaultMode = 'side-by-side',
		hasImage = false,
		minHeightClass = 'min-h-[320px] md:min-h-[420px]',
	}: Props = $props();

	// `defaultMode` seeds the initial state; subsequent updates are driven
	// by the user via the segmented Tabs control. Prop changes after mount
	// don't override user choice on purpose.
	// svelte-ignore state_referenced_locally
	let mode = $state<PreviewMode>(defaultMode);
	let revealValue = $state<number>(50);
</script>

<figure
	class="bg-muted/40 border-border relative flex w-full flex-col overflow-hidden border {minHeightClass}"
	aria-label="Source and processed output comparison preview"
>
	<div class="bg-background/80 border-border flex items-center gap-2 border-b px-2 py-1.5 backdrop-blur">
		<Tabs bind:value={mode} class="shrink-0">
			<TabsList>
				<TabsTrigger value="side-by-side">Side-by-side</TabsTrigger>
				<TabsTrigger value="ab-reveal">A/B reveal</TabsTrigger>
			</TabsList>
		</Tabs>

		<div class="text-muted-foreground ml-auto flex items-center gap-1.5 text-xs">
			<Badge variant="outline" class="gap-1"><MagnifyingGlassIcon /> 100%</Badge>
			<Badge variant="outline" class="hidden sm:inline-flex">512 × 512</Badge>
			<Badge variant="outline" class="hidden md:inline-flex">64 colors</Badge>
		</div>

		<div class="flex shrink-0 items-center gap-1">
			<Button size="icon-sm" variant="ghost" aria-label="Crop">
				<CropIcon />
			</Button>
			<Button size="icon-sm" variant="ghost" aria-label="Fit to view">
				<ArrowsOutIcon />
			</Button>
		</div>
	</div>

	<div class="relative flex flex-1 items-stretch">
		{#if !hasImage}
			<Empty class="flex-1">
				<EmptyHeader>
					<EmptyMedia variant="icon">
						<ImageIcon weight="duotone" />
					</EmptyMedia>
					<EmptyTitle>Drop an image to begin</EmptyTitle>
					<EmptyDescription>
						PNG, JPEG, WebP, or GIF. Everything runs locally — your image never leaves your device.
					</EmptyDescription>
				</EmptyHeader>
				<EmptyContent>
					<Button variant="outline" size="sm">
						<UploadIcon />
						Choose file
					</Button>
				</EmptyContent>
			</Empty>
		{:else if mode === 'side-by-side'}
			<div class="grid flex-1 grid-cols-2 divide-x divide-border">
				{@render pane('Source', 'bg-[repeating-conic-gradient(theme(colors.muted)_0%_25%,transparent_0%_50%)_50%_/_16px_16px]')}
				{@render pane('Output', 'bg-muted')}
			</div>
		{:else}
			<div class="relative flex-1">
				{@render pane('Source · Output', 'bg-muted')}
				<div
					class="bg-foreground/80 absolute top-0 bottom-0 w-px"
					style="left: {revealValue}%"
					aria-hidden="true"
				></div>
				<div
					class="bg-background border-foreground absolute top-1/2 size-8 -translate-x-1/2 -translate-y-1/2 border"
					style="left: {revealValue}%"
					aria-hidden="true"
				></div>
				<div class="absolute inset-x-3 bottom-3 mx-auto max-w-md">
					<Slider
						type="single"
						bind:value={revealValue}
						min={0}
						max={100}
						step={1}
						aria-label="A/B reveal position"
					/>
				</div>
			</div>
		{/if}
	</div>
</figure>

{#snippet pane(label: string, bgClass: string)}
	<div class="relative flex items-center justify-center {bgClass}">
		<span class="bg-background/80 absolute top-2 left-2 px-2 py-0.5 text-xs font-medium">
			{label}
		</span>
	</div>
{/snippet}
