<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import {
		DropdownMenu,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuSeparator,
		DropdownMenuTrigger
	} from '$lib/components/ui/dropdown-menu';
	import { WPLACE_PALETTE, WPLACE_PALETTE_NAME } from '$lib/palette/wplace';
	import { paletteEnabled, setPaletteColorEnabled } from '$lib/stores/app';
	import type { PaletteColor } from '$lib/processing/types';
	import PlusIcon from 'phosphor-svelte/lib/Plus';
	import TrashIcon from 'phosphor-svelte/lib/Trash';
	import DotsIcon from 'phosphor-svelte/lib/DotsThreeVertical';
	import GridIcon from 'phosphor-svelte/lib/GridFour';
	import ListIcon from 'phosphor-svelte/lib/List';
	import LockIcon from 'phosphor-svelte/lib/Lock';
	import PencilIcon from 'phosphor-svelte/lib/PencilSimple';
	import VisibilityCheckbox from './VisibilityCheckbox.svelte';

	type Props = { fillHeight?: boolean };
	let { fillHeight = false }: Props = $props();

	let viewMode = $state<'list' | 'grid'>('list');
	let preset = $state('wplace');
	let gridDisabled = $state(false);
	let selectedColorKeys = $state<Record<string, boolean>>({});

	const selectedCount = $derived(Object.values(selectedColorKeys).filter(Boolean).length);
	const enabledCount = $derived(
		WPLACE_PALETTE.filter((s) => $paletteEnabled[s.key] !== false).length
	);

	$effect(() => {
		if (typeof window === 'undefined') return;
		const mql = window.matchMedia('(any-hover: none), (max-width: 1023.98px)');
		const sync = () => {
			gridDisabled = mql.matches;
			if (mql.matches && viewMode === 'grid') viewMode = 'list';
		};
		sync();
		mql.addEventListener('change', sync);
		return () => mql.removeEventListener('change', sync);
	});

	function setRowSelected(key: string, selected: boolean) {
		selectedColorKeys = { ...selectedColorKeys, [key]: selected };
	}

	function selectAllRows() {
		selectedColorKeys = Object.fromEntries(WPLACE_PALETTE.map((color) => [color.key, true]));
	}

	function deselectRows() {
		selectedColorKeys = {};
	}

	function toggleSelectedVisibility() {
		for (const color of WPLACE_PALETTE) {
			if (!selectedColorKeys[color.key]) continue;
			setPaletteColorEnabled(color.key, $paletteEnabled[color.key] === false);
		}
	}
</script>

<section class="flex flex-col gap-2 {fillHeight ? 'h-full' : ''}" aria-label="Palette editor">
	<div class="flex flex-wrap items-center gap-2">
		<h2 class="text-sm font-semibold tracking-tight">Palette</h2>
		<Select bind:value={preset} type="single">
			<SelectTrigger id="palette-preset" class="h-7 w-auto min-w-0 gap-1 px-2 text-sm font-medium"
				>{WPLACE_PALETTE_NAME}</SelectTrigger
			>
			<SelectContent>
				<SelectItem value="wplace">{WPLACE_PALETTE_NAME}</SelectItem>
			</SelectContent>
		</Select>
		<Badge variant="outline" class="gap-1"><LockIcon /> Built-in</Badge>
	</div>

	<div class="flex flex-wrap items-center gap-1 border border-border bg-background p-1">
		<Button size="xs" variant="ghost" onclick={selectAllRows}>Select all</Button>
		<Button size="xs" variant="ghost" onclick={deselectRows} disabled={selectedCount === 0}
			>Deselect</Button
		>
		<Button
			size="xs"
			variant="ghost"
			onclick={toggleSelectedVisibility}
			disabled={selectedCount === 0}>Toggle visibility</Button
		>
		<div class="ml-auto flex items-center gap-1">
			<Button
				size="xs"
				variant="ghost"
				disabled
				aria-label="Add color (built-in palette is immutable)"
			>
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
					aria-label="Grid view{gridDisabled ? ' (requires a pointer device)' : ''}"
					disabled={gridDisabled}
					title={gridDisabled ? 'Grid view requires a pointer device' : undefined}
				>
					<GridIcon />
				</ToggleGroupItem>
			</ToggleGroup>
		</div>
	</div>

	<ScrollArea
		class="min-h-0 flex-1 border border-border bg-background {fillHeight ? '' : 'max-h-[420px]'}"
	>
		{#if viewMode === 'list'}
			<table class="w-full border-collapse text-xs">
				<thead class="sticky top-0 z-10 bg-muted/50">
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
					{#each WPLACE_PALETTE as color, i (color.key)}
						{@render row(color, i)}
					{/each}
				</tbody>
			</table>
		{:else}
			<div class="grid grid-cols-4 gap-1 p-2 sm:grid-cols-5 md:grid-cols-6 lg:grid-cols-7">
				{#each WPLACE_PALETTE as color, i (color.key)}
					{@render gridSwatch(color, i)}
				{/each}
			</div>
		{/if}
	</ScrollArea>

	<div class="flex items-center justify-between text-xs text-muted-foreground">
		<span>{enabledCount} / {WPLACE_PALETTE.length} colors active</span>
		<span>{selectedCount} selected · Enabled state persists locally.</span>
	</div>
</section>

{#snippet swatchSquare(color: PaletteColor, size: 'sm' | 'md' = 'md')}
	{#if color.kind === 'transparent'}
		<span
			class="block border border-border [background-image:repeating-conic-gradient(rgba(0,0,0,0.3)_0%_25%,transparent_0%_50%)] [background-size:8px_8px] {size ===
			'sm'
				? 'size-5'
				: 'size-7'}"
			aria-label="Transparent"
		></span>
	{:else}
		<span
			class="block border border-border {size === 'sm' ? 'size-5' : 'size-7'}"
			style="background-color: {color.key}"
			aria-hidden="true"
		></span>
	{/if}
{/snippet}

{#snippet kindBadge(color: PaletteColor)}
	{#if color.kind === 'transparent'}
		<Badge variant="outline">Transparent</Badge>
	{:else if color.kind === 'premium'}
		<Badge variant="secondary">Premium</Badge>
	{:else if color.kind === 'custom'}
		<Badge>Custom</Badge>
	{:else}
		<Badge variant="outline">Free</Badge>
	{/if}
{/snippet}

{#snippet row(color: PaletteColor, i: number)}
	{@const enabled = $paletteEnabled[color.key] !== false}
	{@const selected = selectedColorKeys[color.key] === true}
	<tr
		class="border-t border-border hover:bg-muted/40 data-selected:bg-muted/50"
		data-disabled={!enabled || undefined}
		data-selected={selected || undefined}
	>
		<td class="p-1.5 text-muted-foreground tabular-nums">{i + 1}</td>
		<td class="p-1.5">
			<Checkbox
				checked={selected}
				aria-label="Select {color.name}"
				onCheckedChange={(next) => setRowSelected(color.key, next)}
			/>
		</td>
		<td class="p-1.5">
			<VisibilityCheckbox
				checked={enabled}
				aria-label="Visible: {color.name}"
				onCheckedChange={(next) => setPaletteColorEnabled(color.key, next)}
			/>
		</td>
		<td class="p-1.5">{@render swatchSquare(color, 'sm')}</td>
		<td class="truncate p-1.5">{color.name}</td>
		<td class="hidden p-1.5 font-mono text-muted-foreground sm:table-cell"
			>{color.kind === 'transparent' ? '—' : color.key}</td
		>
		<td class="hidden p-1.5 md:table-cell">{@render kindBadge(color)}</td>
		<td class="p-1.5">
			<DropdownMenu>
				<DropdownMenuTrigger aria-label="Row actions"><DotsIcon /></DropdownMenuTrigger>
				<DropdownMenuContent align="end">
					<DropdownMenuItem disabled>Edit…</DropdownMenuItem>
					<DropdownMenuItem disabled>Duplicate</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem disabled>Delete</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</td>
	</tr>
{/snippet}

{#snippet gridSwatch(color: PaletteColor, i: number)}
	{@const enabled = $paletteEnabled[color.key] !== false}
	{@const selected = selectedColorKeys[color.key] === true}
	{@const reveal =
		'opacity-0 transition-opacity group-hover/swatch:opacity-100 group-focus-within/swatch:opacity-100'}
	{@const cornerBtn = `absolute z-10 inline-flex size-5 items-center justify-center bg-background/85 backdrop-blur-[1px] hover:bg-background focus-visible:ring-ring focus-visible:ring-1 focus-visible:outline-none disabled:cursor-not-allowed disabled:hover:bg-background/85 disabled:[&_svg]:opacity-40 [&_svg]:size-3 ${reveal}`}
	{@const cornerCheckbox = `absolute z-10 size-5 rounded-none bg-background/85 backdrop-blur-[1px] ${reveal}`}
	<figure
		class="group/swatch relative aspect-square overflow-hidden border border-border bg-muted data-selected:ring-2 data-selected:ring-primary data-disabled:opacity-40"
		data-disabled={!enabled || undefined}
		data-selected={selected || undefined}
		aria-label="{color.name}{color.kind !== 'transparent' ? ` ${color.key}` : ''}{enabled
			? ''
			: ' (hidden)'}"
		title="{color.name}{color.kind !== 'transparent' ? ` — ${color.key}` : ''}"
	>
		{#if color.kind === 'transparent'}
			<div
				class="absolute inset-0 [background-image:repeating-conic-gradient(rgba(0,0,0,0.3)_0%_25%,transparent_0%_50%)] [background-size:10px_10px]"
				aria-hidden="true"
			></div>
		{:else}
			<div class="absolute inset-0" style="background-color: {color.key}" aria-hidden="true"></div>
		{/if}
		<button
			type="button"
			class="{cornerBtn} top-1 left-1"
			disabled
			aria-label="Edit {color.name}"
			title="Built-in colors are immutable"><PencilIcon /></button
		>
		<button
			type="button"
			class="{cornerBtn} top-1 right-1 hover:text-destructive"
			disabled
			aria-label="Delete {color.name}"
			title="Built-in colors are immutable"><TrashIcon /></button
		>
		<Checkbox
			class="{cornerCheckbox} bottom-1 left-1"
			checked={selected}
			aria-label="Select {color.name}"
			onCheckedChange={(next) => setRowSelected(color.key, next)}
		/>
		<VisibilityCheckbox
			class="{cornerCheckbox} right-1 bottom-1"
			checked={enabled}
			aria-label="Visible: {color.name}"
			onCheckedChange={(next) => setPaletteColorEnabled(color.key, next)}
		/>
		<figcaption class="sr-only">
			{color.name}{color.kind !== 'transparent' ? ` ${color.key}` : ''} · {enabled
				? 'visible'
				: 'hidden'}
		</figcaption>
	</figure>
{/snippet}
