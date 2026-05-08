<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import type { Palette, PaletteColor } from '$lib/processing/types';
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

	function stopSelect(event: Event) {
		event.stopPropagation();
	}
</script>

<div class="grid gap-1.5">
	<h2 class="text-sm font-semibold tracking-tight">Palette</h2>
	<Select bind:value={preset} type="single">
		<SelectTrigger
			id="palette-preset"
			size="auto"
			class="!w-full max-w-full items-center gap-3 p-3 text-left whitespace-normal"
		>
			<span class="grid min-w-0 flex-1 content-start gap-2 overflow-hidden text-left">
				<span class="truncate text-sm font-medium text-foreground">{currentPalette.name}</span>
				{@render swatchStrip(currentPalette.colors)}
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
						<span class="grid min-w-0 flex-1 content-start gap-2 overflow-hidden pr-16">
							<span class="truncate text-sm font-medium text-foreground">{palette.name}</span>
							{@render swatchStrip(palette.colors)}
						</span>
						<span class="ml-auto flex shrink-0 items-center gap-1 pr-5">
							<Button
								type="button"
								variant="ghost"
								size="icon-xs"
								aria-label="Duplicate {palette.name}"
								onpointerdown={stopSelect}
								onclick={(event) => {
									stopSelect(event);
									onDuplicatePalette(palette);
								}}
							>
								<CopyIcon weight="bold" />
							</Button>
							<Button
								type="button"
								variant="ghost"
								size="icon-xs"
								class="hover:text-destructive"
								aria-label="Delete {palette.name}"
								disabled={palette.source === 'wplace'}
								onpointerdown={stopSelect}
								onclick={(event) => {
									stopSelect(event);
									onDeletePalette(palette);
								}}
							>
								<TrashIcon weight="bold" />
							</Button>
						</span>
					</SelectItem>
				{/each}
			</div>
			<div class="border-t border-border p-2">
				<Button variant="ghost" class="w-full justify-start" onclick={onNewPalette}>
					<PlusIcon weight="bold" />
					New palette
				</Button>
				<div class="mt-2 grid grid-cols-2 gap-2">
					<Button variant="outline" onclick={onImportPalette}>
						<UploadIcon weight="bold" />
						Import
					</Button>
					<Button variant="outline" onclick={onExportPalette}>
						<DownloadIcon weight="bold" />
						Export
					</Button>
				</div>
			</div>
		</SelectContent>
	</Select>
</div>

{#snippet swatchStrip(colors: readonly PaletteColor[])}
	<span class="flex max-w-full min-w-0 flex-nowrap gap-1 overflow-hidden" aria-hidden="true">
		{#each colors as color (color.key)}
			<span class="size-4 shrink-0 border border-border" title={color.name}>
				{#if color.kind === 'transparent'}
					<span
						class="block size-full [background-image:repeating-conic-gradient(rgba(0,0,0,0.3)_0%_25%,transparent_0%_50%)] [background-size:6px_6px]"
					></span>
				{:else}
					<span class="block size-full" style="background-color: {color.key}"></span>
				{/if}
			</span>
		{/each}
	</span>
{/snippet}
