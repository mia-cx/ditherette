<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import type { Palette } from '$lib/processing/types';
	import PaletteSwatchStrip from './PaletteSwatchStrip.svelte';
	import CopyIcon from 'phosphor-svelte/lib/Copy';
	import DownloadIcon from 'phosphor-svelte/lib/DownloadSimple';
	import PlusIcon from 'phosphor-svelte/lib/Plus';
	import TrashIcon from 'phosphor-svelte/lib/Trash';
	import UploadIcon from 'phosphor-svelte/lib/UploadSimple';

	type Props = {
		preset: string;
		palettes: readonly Palette[];
		currentPalette: Palette;
		onNewPalette: () => void;
		onDuplicatePalette: (palette: Palette) => void;
		onDeletePalette: (palette: Palette) => void;
		onImportPalette: () => void;
		onExportPalette: () => void;
	};

	let {
		preset = $bindable(),
		palettes,
		currentPalette,
		onNewPalette,
		onDuplicatePalette,
		onDeletePalette,
		onImportPalette,
		onExportPalette
	}: Props = $props();

	let open = $state(false);

	const actionPalette = $derived(
		palettes.find((palette) => palette.name === preset) ?? currentPalette
	);
</script>

<div class="grid gap-1.5">
	<h2 class="text-sm font-semibold tracking-tight">Palette</h2>
	<Select bind:open bind:value={preset} type="single">
		<SelectTrigger
			id="palette-preset"
			size="auto"
			class="!w-full max-w-full items-center gap-3 p-3 text-left whitespace-normal"
		>
			<span class="grid min-w-0 flex-1 content-start gap-2 overflow-hidden text-left">
				<span class="truncate text-sm font-medium text-foreground">{currentPalette.name}</span>
				<PaletteSwatchStrip colors={currentPalette.colors} />
			</span>
		</SelectTrigger>
		<SelectContent
			viewportFlex
			class="max-h-[min(32rem,var(--bits-select-content-available-height))] w-(--bits-select-anchor-width) max-w-(--bits-select-anchor-width) overflow-hidden p-0"
		>
			<div class="min-h-0 overflow-y-auto py-1">
				{#each palettes as palette (palette.name)}
					<SelectItem
						value={palette.name}
						label={palette.name}
						class="min-w-0 items-start py-3 pr-8 pl-3"
					>
						<span class="grid min-w-0 flex-1 content-start gap-2 overflow-hidden">
							<span class="truncate text-sm font-medium text-foreground">{palette.name}</span>
							<PaletteSwatchStrip colors={palette.colors} />
						</span>
					</SelectItem>
				{/each}
			</div>
			<div class="border-t border-border p-2">
				<div class="grid grid-cols-2 gap-2">
					<Button
						variant="outline"
						aria-label="Duplicate {actionPalette.name}"
						onclick={() => {
							onDuplicatePalette(actionPalette);
							open = false;
						}}
					>
						<CopyIcon weight="bold" />
						Duplicate
					</Button>
					<Button
						variant="outline"
						class="hover:text-destructive"
						aria-label="Delete {actionPalette.name}"
						disabled={actionPalette.source === 'wplace'}
						onclick={() => {
							onDeletePalette(actionPalette);
							open = false;
						}}
					>
						<TrashIcon weight="bold" />
						Delete
					</Button>
				</div>
				<Button
					variant="ghost"
					class="mt-2 w-full justify-start"
					onclick={() => {
						onNewPalette();
						open = false;
					}}
				>
					<PlusIcon weight="bold" />
					New palette
				</Button>
				<div class="mt-2 grid grid-cols-2 gap-2">
					<Button
						variant="outline"
						onclick={() => {
							onImportPalette();
							open = false;
						}}
					>
						<UploadIcon weight="bold" />
						Import
					</Button>
					<Button
						variant="outline"
						onclick={() => {
							onExportPalette();
							open = false;
						}}
					>
						<DownloadIcon weight="bold" />
						Export
					</Button>
				</div>
			</div>
		</SelectContent>
	</Select>
</div>
