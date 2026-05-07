<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import {
		Select,
		SelectContent,
		SelectItem,
		SelectTrigger,
	} from '$lib/components/ui/select';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import {
		DropdownMenu,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuSeparator,
		DropdownMenuTrigger,
	} from '$lib/components/ui/dropdown-menu';
	import PlusIcon from 'phosphor-svelte/lib/Plus';
	import TrashIcon from 'phosphor-svelte/lib/Trash';
	import DotsIcon from 'phosphor-svelte/lib/DotsThreeVertical';
	import GridIcon from 'phosphor-svelte/lib/GridFour';
	import ListIcon from 'phosphor-svelte/lib/List';
	import LockIcon from 'phosphor-svelte/lib/Lock';
	import PencilIcon from 'phosphor-svelte/lib/PencilSimple';
	import VisibilityCheckbox from './VisibilityCheckbox.svelte';
	import { SAMPLE_PALETTE, type Swatch } from './sample-data';

	type Props = {
		fillHeight?: boolean;
	};

	let { fillHeight = false }: Props = $props();

	let viewMode = $state<'list' | 'grid'>('list');
	let preset = $state('wplace');
	let isMobile = $state(false);

	const enabledCount = $derived(SAMPLE_PALETTE.filter((s) => s.enabled).length);

	// Grid view is intentionally disabled on mobile: each swatch carries
	// hover-only action buttons that aren't reachable without a pointer.
	// Track viewport, force list view at narrow widths, and disable the
	// grid toggle so users can't strand themselves there.
	$effect(() => {
		if (typeof window === 'undefined') return;
		const mql = window.matchMedia('(max-width: 767.98px)');
		const sync = () => {
			isMobile = mql.matches;
			if (mql.matches && viewMode === 'grid') viewMode = 'list';
		};
		sync();
		mql.addEventListener('change', sync);
		return () => mql.removeEventListener('change', sync);
	});
</script>

<section
	class="flex flex-col gap-2 {fillHeight ? 'h-full' : ''}"
	aria-label="Palette editor"
>
	<!-- Combined header: panel title + inline preset dropdown + built-in badge.
	     Replaces the previous separate "Palette" heading and "Preset" label row
	     to give the actual palette table more vertical room. -->
	<div class="flex flex-wrap items-center gap-2">
		<h2 class="text-sm font-semibold tracking-tight">Palette</h2>
		<Select bind:value={preset} type="single">
			<SelectTrigger
				id="palette-preset"
				class="h-7 w-auto min-w-0 gap-1 px-2 text-sm font-medium"
			>
				Wplace (Default)
			</SelectTrigger>
			<SelectContent>
				<SelectItem value="wplace">Wplace (Default)</SelectItem>
				<SelectItem value="custom-1">Custom · Sunset</SelectItem>
				<SelectItem value="custom-2">Custom · Greyscale</SelectItem>
			</SelectContent>
		</Select>
		<Badge variant="outline" class="gap-1"><LockIcon /> Built-in</Badge>
	</div>

	<div class="border-border bg-background flex flex-wrap items-center gap-1 border p-1">
		<Button size="xs" variant="ghost">Select all</Button>
		<Button size="xs" variant="ghost">Deselect</Button>
		<Button size="xs" variant="ghost">Toggle</Button>
		<div class="ml-auto flex items-center gap-1">
			<Button size="xs" variant="ghost" disabled aria-label="Add color (built-in palette is immutable)">
				<PlusIcon />
				<span class="hidden sm:inline">Add</span>
			</Button>
			<Button size="xs" variant="ghost" disabled aria-label="Delete selected colors">
				<TrashIcon />
				<span class="hidden sm:inline">Delete</span>
			</Button>
			<ToggleGroup
				type="single"
				bind:value={viewMode}
				size="sm"
				variant="outline"
				aria-label="Palette view mode"
			>
				<ToggleGroupItem value="list" aria-label="List view"><ListIcon /></ToggleGroupItem>
				<ToggleGroupItem
					value="grid"
					aria-label="Grid view{isMobile ? ' (disabled on mobile)' : ''}"
					disabled={isMobile}
					title={isMobile ? 'Grid view requires a pointer device' : undefined}
				>
					<GridIcon />
				</ToggleGroupItem>
			</ToggleGroup>
		</div>
	</div>

	<ScrollArea
		class="border-border bg-background min-h-0 flex-1 border {fillHeight ? '' : 'max-h-[420px]'}"
	>
		{#if viewMode === 'list'}
			<table class="w-full border-collapse text-xs">
				<thead class="bg-muted/50 sticky top-0 z-10">
					<tr class="text-muted-foreground">
						<th class="w-8 p-1.5 text-left font-medium">#</th>
						<th class="w-8 p-1.5"><span class="sr-only">Select</span></th>
						<th class="w-8 p-1.5"><span class="sr-only">Visible</span></th>
						<th class="w-12 p-1.5 text-left font-medium">Swatch</th>
						<th class="p-1.5 text-left font-medium">Name</th>
						<th class="hidden p-1.5 text-left font-medium sm:table-cell">Hex</th>
						<th class="hidden p-1.5 text-left font-medium md:table-cell">Status</th>
						<th class="w-8 p-1.5"><span class="sr-only">Actions</span></th>
					</tr>
				</thead>
				<tbody>
					{#each SAMPLE_PALETTE as color, i (i)}
						{@render row(color, i)}
					{/each}
				</tbody>
			</table>
		{:else}
			<div class="grid grid-cols-4 gap-1 p-2 sm:grid-cols-5 md:grid-cols-6 lg:grid-cols-7">
				{#each SAMPLE_PALETTE as color, i (i)}
					{@render gridSwatch(color, i)}
				{/each}
			</div>
		{/if}
	</ScrollArea>

	<div class="text-muted-foreground flex items-center justify-between text-xs">
		<span>{enabledCount} / {SAMPLE_PALETTE.length} colors active</span>
		<Button variant="ghost" size="xs">Manage…</Button>
	</div>
</section>

{#snippet swatchSquare(color: Swatch, size: 'sm' | 'md' = 'md')}
	{#if color.kind === 'transparent'}
		<span
			class="border-border block border [background-image:repeating-conic-gradient(rgba(0,0,0,0.3)_0%_25%,transparent_0%_50%)] [background-size:8px_8px] {size ===
			'sm'
				? 'size-5'
				: 'size-7'}"
			aria-label="Transparent"
		></span>
	{:else}
		<span
			class="border-border block border {size === 'sm' ? 'size-5' : 'size-7'}"
			style="background-color: {color.hex}"
			aria-hidden="true"
		></span>
	{/if}
{/snippet}

{#snippet row(color: Swatch, i: number)}
	<tr class="border-border border-t hover:bg-muted/40" data-disabled={!color.enabled || undefined}>
		<td class="text-muted-foreground p-1.5 tabular-nums">{i + 1}</td>
		<td class="p-1.5"><Checkbox aria-label="Select {color.name}" /></td>
		<td class="p-1.5">
			<VisibilityCheckbox checked={color.enabled} aria-label="Visible: {color.name}" />
		</td>
		<td class="p-1.5">{@render swatchSquare(color, 'sm')}</td>
		<td class="truncate p-1.5">{color.name}</td>
		<td class="text-muted-foreground hidden p-1.5 font-mono sm:table-cell">{color.hex ?? '—'}</td>
		<td class="hidden p-1.5 md:table-cell">
			{#if color.kind === 'transparent'}
				<Badge variant="outline">Transparent</Badge>
			{:else if color.kind === 'premium'}
				<Badge variant="secondary">Premium</Badge>
			{:else if color.kind === 'custom'}
				<Badge>Custom</Badge>
			{:else}
				<Badge variant="outline">Free</Badge>
			{/if}
		</td>
		<td class="p-1.5">
			<DropdownMenu>
				<DropdownMenuTrigger aria-label="Row actions">
					<DotsIcon />
				</DropdownMenuTrigger>
				<DropdownMenuContent align="end">
					<DropdownMenuItem>Edit…</DropdownMenuItem>
					<DropdownMenuItem>Duplicate</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem disabled>Delete</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</td>
	</tr>
{/snippet}

<!--
	Grid swatch with four corner controls:
	  TL = edit, TR = delete, BL = select (bulk), BR = visible toggle (eye).
	The colored swatch fills the cell as the background; controls overlay
	the corners with a translucent backdrop so the colour stays readable.
-->
{#snippet gridSwatch(color: Swatch, i: number)}
	{@const builtIn = color.kind !== 'custom'}
	{@const reveal =
		'opacity-0 transition-opacity group-hover/swatch:opacity-100 group-focus-within/swatch:opacity-100'}
	{@const cornerBtn =
		`absolute z-10 inline-flex size-5 items-center justify-center bg-background/85 backdrop-blur-[1px] hover:bg-background focus-visible:ring-ring focus-visible:ring-1 focus-visible:outline-none disabled:cursor-not-allowed disabled:hover:bg-background/85 disabled:[&_svg]:opacity-40 [&_svg]:size-3 ${reveal}`}
	{@const cornerCheckbox =
		`absolute z-10 size-5 rounded-none bg-background/85 backdrop-blur-[1px] ${reveal}`}
	<figure
		class="border-border bg-muted group/swatch relative aspect-square overflow-hidden border data-disabled:opacity-40"
		data-disabled={!color.enabled || undefined}
		aria-label="{color.name}{color.hex ? ` ${color.hex}` : ''}{color.enabled ? '' : ' (hidden)'}"
		title="{color.name}{color.hex ? ` — ${color.hex}` : ''}"
	>
		{#if color.kind === 'transparent'}
			<div
				class="absolute inset-0 [background-image:repeating-conic-gradient(rgba(0,0,0,0.3)_0%_25%,transparent_0%_50%)] [background-size:10px_10px]"
				aria-hidden="true"
			></div>
		{:else}
			<div
				class="absolute inset-0"
				style="background-color: {color.hex}"
				aria-hidden="true"
			></div>
		{/if}

		<button
			type="button"
			class="{cornerBtn} top-1 left-1"
			disabled={builtIn}
			aria-label="Edit {color.name}"
			title={builtIn ? 'Built-in colors are immutable' : `Edit ${color.name}`}
		>
			<PencilIcon />
		</button>

		<button
			type="button"
			class="{cornerBtn} top-1 right-1 hover:text-destructive"
			disabled={builtIn}
			aria-label="Delete {color.name}"
			title={builtIn ? 'Built-in colors are immutable' : `Delete ${color.name}`}
		>
			<TrashIcon />
		</button>

		<Checkbox
			class="{cornerCheckbox} bottom-1 left-1"
			aria-label="Select {color.name}"
		/>

		<VisibilityCheckbox
			class="{cornerCheckbox} right-1 bottom-1"
			checked={color.enabled}
			aria-label="Visible: {color.name}"
		/>

		<figcaption class="sr-only">
			{color.name}{color.hex ? ` ${color.hex}` : ''} · {color.enabled ? 'visible' : 'hidden'}
		</figcaption>
	</figure>
{/snippet}
