<script lang="ts">
	import { Label } from '$lib/components/ui/label';
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
	import { SAMPLE_PALETTE, type Swatch } from './sample-data';

	type Props = {
		fillHeight?: boolean;
		hideHeading?: boolean;
	};

	let { fillHeight = false, hideHeading = false }: Props = $props();

	let viewMode = $state<'list' | 'grid'>('list');
	let preset = $state('wplace');

	const enabledCount = $derived(SAMPLE_PALETTE.filter((s) => s.enabled).length);
</script>

<section
	class="flex flex-col gap-3 {fillHeight ? 'h-full' : ''}"
	aria-label="Palette editor"
>
	{#if !hideHeading}
		<div class="flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold tracking-tight">Palette</h2>
			<p class="text-muted-foreground text-xs">Preset, enabled colors, and edits.</p>
		</div>
	{/if}

	<div class="grid gap-1.5">
		<Label for="palette-preset">Preset</Label>
		<div class="flex items-center gap-2">
			<Select bind:value={preset} type="single">
				<SelectTrigger id="palette-preset" class="flex-1">Wplace (Default)</SelectTrigger>
				<SelectContent>
					<SelectItem value="wplace">Wplace (Default)</SelectItem>
					<SelectItem value="custom-1">Custom · Sunset</SelectItem>
					<SelectItem value="custom-2">Custom · Greyscale</SelectItem>
				</SelectContent>
			</Select>
			<Badge variant="outline" class="gap-1"><LockIcon /> Built-in</Badge>
		</div>
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
				<ToggleGroupItem value="grid" aria-label="Grid view"><GridIcon /></ToggleGroupItem>
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
						<th class="w-8 p-1.5"><span class="sr-only">Enabled</span></th>
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
			<div class="grid grid-cols-6 gap-1 p-2 sm:grid-cols-8 md:grid-cols-10">
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
	<tr class="border-border border-t hover:bg-muted/40" data-disabled={!color.enabled}>
		<td class="text-muted-foreground p-1.5 tabular-nums">{i + 1}</td>
		<td class="p-1.5"><Checkbox aria-label="Select {color.name}" /></td>
		<td class="p-1.5"><Checkbox checked={color.enabled} aria-label="Enable {color.name}" /></td>
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

{#snippet gridSwatch(color: Swatch, i: number)}
	<button
		type="button"
		class="border-border focus-visible:ring-ring flex aspect-square items-center justify-center border focus-visible:ring-2 focus-visible:outline-none data-disabled:opacity-30"
		data-disabled={!color.enabled || undefined}
		aria-label="{color.name} {color.hex ?? ''} {color.enabled ? '' : '(disabled)'}"
		title="{color.name}{color.hex ? ` — ${color.hex}` : ''}"
	>
		{@render swatchSquare(color, 'md')}
	</button>
{/snippet}
