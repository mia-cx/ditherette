<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import type { Palette } from '$lib/processing/types';
	import CopyIcon from 'phosphor-svelte/lib/Copy';
	import DownloadIcon from 'phosphor-svelte/lib/DownloadSimple';
	import GridIcon from 'phosphor-svelte/lib/GridFour';
	import ListIcon from 'phosphor-svelte/lib/List';
	import LockIcon from 'phosphor-svelte/lib/Lock';
	import PlusIcon from 'phosphor-svelte/lib/Plus';
	import TrashIcon from 'phosphor-svelte/lib/Trash';
	import UploadIcon from 'phosphor-svelte/lib/UploadSimple';

	type Props = {
		preset: string;
		viewMode: 'list' | 'grid';
		palettes: readonly Palette[];
		currentPalette: Palette;
		isBuiltIn: boolean;
		selectedCount: number;
		gridDisabled: boolean;
		onSelectAll: () => void;
		onDeselect: () => void;
		onToggleSelectedVisibility: () => void;
		onNewPalette: () => void;
		onDuplicatePalette: () => void;
		onImportPalette: () => void;
		onExportPalette: () => void;
		onAddColor: () => void;
		onDeleteSelectedColors: () => void;
	};

	let {
		preset = $bindable(),
		viewMode = $bindable(),
		palettes,
		currentPalette,
		isBuiltIn,
		selectedCount,
		gridDisabled,
		onSelectAll,
		onDeselect,
		onToggleSelectedVisibility,
		onNewPalette,
		onDuplicatePalette,
		onImportPalette,
		onExportPalette,
		onAddColor,
		onDeleteSelectedColors
	}: Props = $props();
</script>

<div class="flex flex-wrap items-center gap-2">
	<h2 class="text-sm font-semibold tracking-tight">Palette</h2>
	<Select bind:value={preset} type="single">
		<SelectTrigger id="palette-preset" class="h-7 w-auto min-w-0 gap-1 px-2 text-sm font-medium">
			{currentPalette.name}
		</SelectTrigger>
		<SelectContent>
			{#each palettes as palette (palette.name)}
				<SelectItem value={palette.name}>{palette.name}</SelectItem>
			{/each}
		</SelectContent>
	</Select>
	<Badge variant="outline" class="gap-1">
		{#if isBuiltIn}<LockIcon weight="bold" /> Built-in{:else}Custom{/if}
	</Badge>
</div>

<div class="flex flex-wrap items-center gap-1 border border-border bg-background p-1">
	<Button size="xs" variant="ghost" onclick={onSelectAll}>Select all</Button>
	<Button size="xs" variant="ghost" onclick={onDeselect} disabled={selectedCount === 0}
		>Deselect</Button
	>
	<Button
		size="xs"
		variant="ghost"
		onclick={onToggleSelectedVisibility}
		disabled={selectedCount === 0}
	>
		Toggle visibility
	</Button>
	<Button size="xs" variant="ghost" onclick={onNewPalette}>New</Button>
	<Button size="xs" variant="ghost" onclick={onDuplicatePalette}
		><CopyIcon weight="bold" /> Duplicate</Button
	>
	<Button size="xs" variant="ghost" onclick={onImportPalette}
		><UploadIcon weight="bold" /> Import</Button
	>
	<Button size="xs" variant="ghost" onclick={onExportPalette}
		><DownloadIcon weight="bold" /> Export</Button
	>
	<div class="ml-auto flex items-center gap-1">
		<Button size="xs" variant="ghost" onclick={onAddColor} aria-label="Add color">
			<PlusIcon weight="bold" />
			<span class="hidden sm:inline">Add</span>
		</Button>
		<Button
			size="xs"
			variant="ghost"
			onclick={onDeleteSelectedColors}
			disabled={selectedCount === 0}
			aria-label="Delete selected colors"
		>
			<TrashIcon weight="bold" />
			<span class="hidden sm:inline">Delete</span>
		</Button>
		<ToggleGroup
			type="single"
			bind:value={viewMode}
			size="sm"
			variant="outline"
			aria-label="Palette view mode"
		>
			<ToggleGroupItem value="list" aria-label="List view"
				><ListIcon weight="bold" /></ToggleGroupItem
			>
			<ToggleGroupItem
				value="grid"
				aria-label="Grid view{gridDisabled ? ' (requires a pointer device)' : ''}"
				disabled={gridDisabled}
				title={gridDisabled ? 'Grid view requires a pointer device' : undefined}
			>
				<GridIcon weight="bold" />
			</ToggleGroupItem>
		</ToggleGroup>
	</div>
</div>
