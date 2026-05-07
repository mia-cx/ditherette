<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { Slider } from '$lib/components/ui/slider';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogFooter,
		DialogHeader,
		DialogTitle
	} from '$lib/components/ui/dialog';
	import {
		DropdownMenu,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuSeparator,
		DropdownMenuTrigger
	} from '$lib/components/ui/dropdown-menu';
	import { TRANSPARENT_KEY, paletteEnabledKey } from '$lib/palette/wplace';
	import {
		activePalette,
		activePaletteName,
		addColorToActivePalette,
		createCustomPalette,
		customPalettes,
		deleteActiveCustomPalette,
		deleteActivePaletteColors,
		duplicateActivePalette,
		duplicateActivePaletteColor,
		editActivePaletteColor,
		importCustomPaletteData,
		paletteEnabled,
		previewCustomPaletteImport,
		outputSettings,
		palettes,
		selectedPalette,
		setPaletteColorEnabled,
		updateOutputSettings
	} from '$lib/stores/app';
	import type { Palette, PaletteColor } from '$lib/processing/types';
	import { ALPHA_MODES } from './sample-data';
	import PlusIcon from 'phosphor-svelte/lib/Plus';
	import TrashIcon from 'phosphor-svelte/lib/Trash';
	import DotsIcon from 'phosphor-svelte/lib/DotsThreeVertical';
	import GridIcon from 'phosphor-svelte/lib/GridFour';
	import ListIcon from 'phosphor-svelte/lib/List';
	import LockIcon from 'phosphor-svelte/lib/Lock';
	import PencilIcon from 'phosphor-svelte/lib/PencilSimple';
	import CopyIcon from 'phosphor-svelte/lib/Copy';
	import UploadIcon from 'phosphor-svelte/lib/UploadSimple';
	import DownloadIcon from 'phosphor-svelte/lib/DownloadSimple';
	import VisibilityCheckbox from './VisibilityCheckbox.svelte';

	type Props = { fillHeight?: boolean };
	let { fillHeight = false }: Props = $props();

	let viewMode = $state<'list' | 'grid'>('list');
	// Bound select state is intentionally local; effects synchronize it with the persistent nanostore.
	// eslint-disable-next-line svelte/prefer-writable-derived
	let preset = $state(activePaletteName.get());
	let gridDisabled = $state(false);
	let selectedColorKeys = $state<Record<string, boolean>>({});
	let importInput = $state<HTMLInputElement>();
	let paletteMessage = $state<string>();
	let paletteMessageTone = $state<'neutral' | 'error'>('neutral');
	let colorDialogOpen = $state(false);
	let colorDialogMode = $state<'add' | 'edit' | 'duplicate'>('add');
	let colorDialogColor = $state<PaletteColor>();
	let colorName = $state('');
	let colorHex = $state('#FF00AA');
	let deleteDialogOpen = $state(false);
	let deleteColorKey = $state<string>();
	let importConfirmOpen = $state(false);
	let pendingImportData = $state<unknown>();
	let pendingImportSummary = $state<
		{ name: string; existingCount: number; importedCount: number }[]
	>([]);
	const initialOutputSettings = outputSettings.get();
	let alpha = $state(initialOutputSettings.alphaMode);
	let alphaThreshold = $state<number>(initialOutputSettings.alphaThreshold);
	let matteKey = $state(initialOutputSettings.matteKey ?? '#FFFFFF');

	const currentPalette = $derived($activePalette);
	const isBuiltIn = $derived(currentPalette.source === 'wplace');
	const selectedCount = $derived(Object.values(selectedColorKeys).filter(Boolean).length);
	const enabledCount = $derived(
		currentPalette.colors.filter(
			(color) => $paletteEnabled[paletteEnabledKey(currentPalette.name, color.key)] !== false
		).length
	);
	const transparentEnabled = $derived(
		$paletteEnabled[paletteEnabledKey(currentPalette.name, TRANSPARENT_KEY)] !== false
	);
	const alphaLabel = $derived(ALPHA_MODES.find((mode) => mode.id === alpha)?.label ?? 'Alpha');
	const visiblePaletteColors = $derived($selectedPalette.filter((color) => color.rgb));
	const matteLabel = $derived(
		visiblePaletteColors.find((color) => color.key === matteKey)?.name ?? 'Select matte color'
	);

	$effect(() => {
		activePaletteName.set(preset);
		selectedColorKeys = {};
	});

	$effect(() => {
		preset = $activePalette.name;
	});

	$effect(() =>
		outputSettings.subscribe((settings) => {
			alpha = settings.alphaMode;
			alphaThreshold = settings.alphaThreshold;
			matteKey = settings.matteKey ?? '#FFFFFF';
		})
	);

	$effect(() => {
		updateOutputSettings({ alphaMode: alpha, alphaThreshold, matteKey });
	});

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

	function enabled(color: PaletteColor, palette: Palette = currentPalette) {
		return $paletteEnabled[paletteEnabledKey(palette.name, color.key)] !== false;
	}

	function setRowSelected(key: string, selected: boolean) {
		selectedColorKeys = { ...selectedColorKeys, [key]: selected };
	}

	function selectAllRows() {
		selectedColorKeys = Object.fromEntries(currentPalette.colors.map((color) => [color.key, true]));
	}

	function deselectRows() {
		selectedColorKeys = {};
	}

	function selectedKeys() {
		return Object.entries(selectedColorKeys)
			.filter(([, selected]) => selected)
			.map(([key]) => key);
	}

	function toggleSelectedVisibility() {
		const colors = currentPalette.colors.filter((color) => selectedColorKeys[color.key]);
		const nextEnabled = colors.some((color) => !enabled(color));
		for (const color of colors) setPaletteColorEnabled(color.key, nextEnabled, currentPalette.name);
	}

	function useDarkestPaletteColor() {
		const darkest = visiblePaletteColors.reduce<(typeof visiblePaletteColors)[number] | undefined>(
			(best, color) => {
				if (!color.rgb) return best;
				if (!best?.rgb) return color;
				return relativeLuminance(color.rgb) < relativeLuminance(best.rgb) ? color : best;
			},
			undefined
		);
		if (darkest) matteKey = darkest.key;
	}

	function relativeLuminance(rgb: { r: number; g: number; b: number }) {
		return 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
	}

	function setPaletteMessage(message: string | undefined, tone: 'neutral' | 'error' = 'neutral') {
		paletteMessage = message;
		paletteMessageTone = tone;
	}

	function withPaletteError(run: () => void) {
		try {
			run();
			setPaletteMessage(undefined);
		} catch (error) {
			setPaletteMessage(error instanceof Error ? error.message : 'Palette action failed.', 'error');
		}
	}

	function newPalette() {
		const name = prompt('Custom palette name', 'My palette');
		if (!name) return;
		withPaletteError(() => createCustomPalette(name));
	}

	function duplicatePalette() {
		const name = prompt('Duplicate palette as', `${currentPalette.name} Copy`);
		if (!name) return;
		withPaletteError(() => duplicateActivePalette(name));
	}

	function openColorDialog(mode: 'add' | 'edit' | 'duplicate', color?: PaletteColor) {
		if (isBuiltIn || color?.kind === 'transparent') {
			setPaletteMessage('Built-in and transparent colors cannot be edited.', 'error');
			return;
		}
		colorDialogMode = mode;
		colorDialogColor = color;
		colorName = color ? `${color.name}${mode === 'duplicate' ? ' Copy' : ''}` : '';
		colorHex = color?.key ?? '#FF00AA';
		colorDialogOpen = true;
	}

	function saveColorDialog() {
		withPaletteError(() => {
			if (colorDialogMode === 'edit' && colorDialogColor) {
				editActivePaletteColor(colorDialogColor.key, colorName, colorHex);
			} else if (colorDialogMode === 'duplicate' && colorDialogColor) {
				duplicateActivePaletteColor(colorDialogColor.key, colorName, colorHex);
			} else {
				addColorToActivePalette(colorName, colorHex);
			}
		});
		if (!paletteMessage) colorDialogOpen = false;
	}

	function addColor() {
		if (isBuiltIn) {
			setPaletteMessage(
				'Built-in palettes are immutable. Duplicate Wplace before adding colors.',
				'error'
			);
			return;
		}
		openColorDialog('add');
	}

	function editColor(color: PaletteColor) {
		openColorDialog('edit', color);
	}

	function duplicateColor(color: PaletteColor) {
		openColorDialog('duplicate', color);
	}

	function deleteSelectedColors() {
		if (isBuiltIn) {
			setPaletteMessage(
				'Built-in palettes are immutable. Duplicate Wplace before deleting colors.',
				'error'
			);
			return;
		}
		const keys = selectedKeys();
		if (!keys.length) return;
		if (!confirm(`Delete ${keys.length} selected color(s) from ${currentPalette.name}?`)) return;
		withPaletteError(() => deleteActivePaletteColors(keys));
		deselectRows();
	}

	function deleteColor(color: PaletteColor) {
		if (isBuiltIn || color.kind === 'transparent') {
			setPaletteMessage('Built-in and transparent colors cannot be deleted.', 'error');
			return;
		}
		deleteColorKey = color.key;
		deleteDialogOpen = true;
	}

	function confirmDeleteColor() {
		const key = deleteColorKey;
		if (!key) return;
		withPaletteError(() => deleteActivePaletteColors([key]));
		deleteDialogOpen = false;
		deleteColorKey = undefined;
	}

	function deletePalette() {
		if (isBuiltIn) {
			setPaletteMessage('The Wplace palette cannot be deleted.', 'error');
			return;
		}
		if (!confirm(`Delete custom palette ${currentPalette.name}?`)) return;
		withPaletteError(deleteActiveCustomPalette);
	}

	function exportPalette(all = false) {
		const data = all
			? $palettes.filter((palette) => palette.source === 'custom')
			: [currentPalette].filter((palette) => palette.source === 'custom');
		if (!data.length) {
			setPaletteMessage('There are no custom palettes to export.', 'error');
			return;
		}
		const blob = new Blob([JSON.stringify(all ? data : data[0], null, 2)], {
			type: 'application/json'
		});
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = all ? 'ditherette-palettes.json' : `${data[0].name}.palette.json`;
		link.click();
		URL.revokeObjectURL(url);
	}

	async function importPalette(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		try {
			const data = JSON.parse(await file.text());
			const preview = previewCustomPaletteImport(data);
			if (preview.overwrites.length) {
				pendingImportData = data;
				pendingImportSummary = preview.overwrites.map((name) => {
					const existing = $customPalettes.find((palette) => palette.name === name)!;
					const imported = preview.palettes.find((palette) => palette.name === name)!;
					return {
						name,
						existingCount: existing.colors.length,
						importedCount: imported.colors.length
					};
				});
				importConfirmOpen = true;
				return;
			}
			applyPaletteImport(data);
		} catch (error) {
			setPaletteMessage(
				error instanceof Error ? error.message : 'Could not import palette JSON.',
				'error'
			);
		} finally {
			input.value = '';
		}
	}

	function applyPaletteImport(data: unknown) {
		const count = importCustomPaletteData(data);
		setPaletteMessage(`Imported ${count} custom palette${count === 1 ? '' : 's'}.`);
	}

	function confirmPaletteImport() {
		if (pendingImportData === undefined) return;
		try {
			applyPaletteImport(pendingImportData);
			importConfirmOpen = false;
			pendingImportData = undefined;
			pendingImportSummary = [];
		} catch (error) {
			setPaletteMessage(
				error instanceof Error ? error.message : 'Could not import palette JSON.',
				'error'
			);
		}
	}
</script>

<input
	bind:this={importInput}
	class="sr-only"
	type="file"
	accept="application/json"
	onchange={importPalette}
/>

<Dialog bind:open={colorDialogOpen}>
	<DialogContent>
		<DialogHeader>
			<DialogTitle>
				{colorDialogMode === 'edit'
					? 'Edit color'
					: colorDialogMode === 'duplicate'
						? 'Duplicate color'
						: 'Add color'}
			</DialogTitle>
			<DialogDescription>
				Custom palettes support up to 256 entries. Hex values must be unique within the palette.
			</DialogDescription>
		</DialogHeader>
		<div class="grid gap-3 py-2">
			<div class="grid gap-1.5">
				<Label for="palette-color-name">Name</Label>
				<Input id="palette-color-name" bind:value={colorName} placeholder="Sky blue" />
			</div>
			<div class="grid gap-1.5">
				<Label for="palette-color-hex">Hex</Label>
				<Input id="palette-color-hex" bind:value={colorHex} placeholder="#66AAFF" />
			</div>
		</div>
		<DialogFooter>
			<Button variant="outline" onclick={() => (colorDialogOpen = false)}>Cancel</Button>
			<Button onclick={saveColorDialog}>Save color</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>

<Dialog bind:open={deleteDialogOpen}>
	<DialogContent>
		<DialogHeader>
			<DialogTitle>Delete color?</DialogTitle>
			<DialogDescription>
				This removes the custom color from {currentPalette.name}. Built-in Wplace colors remain
				immutable.
			</DialogDescription>
		</DialogHeader>
		<DialogFooter>
			<Button variant="outline" onclick={() => (deleteDialogOpen = false)}>Cancel</Button>
			<Button variant="destructive" onclick={confirmDeleteColor}>Delete</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>

<Dialog bind:open={importConfirmOpen}>
	<DialogContent>
		<DialogHeader>
			<DialogTitle>Replace custom palettes?</DialogTitle>
			<DialogDescription>
				Importing this file will replace existing custom palettes with matching names. Enabled state
				is preserved by matching colors where possible.
			</DialogDescription>
		</DialogHeader>
		<ul class="grid gap-2 py-2 text-sm">
			{#each pendingImportSummary as item (item.name)}
				<li class="rounded border border-border bg-muted/40 px-3 py-2">
					<span class="font-medium">{item.name}</span>
					<span class="text-muted-foreground">
						— {item.existingCount} existing colors → {item.importedCount} imported colors
					</span>
				</li>
			{/each}
		</ul>
		<DialogFooter>
			<Button variant="outline" onclick={() => (importConfirmOpen = false)}>Cancel</Button>
			<Button onclick={confirmPaletteImport}>Replace and import</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>

<section class="flex flex-col gap-2 {fillHeight ? 'h-full' : ''}" aria-label="Palette editor">
	<div class="flex flex-wrap items-center gap-2">
		<h2 class="text-sm font-semibold tracking-tight">Palette</h2>
		<Select bind:value={preset} type="single">
			<SelectTrigger id="palette-preset" class="h-7 w-auto min-w-0 gap-1 px-2 text-sm font-medium">
				{currentPalette.name}
			</SelectTrigger>
			<SelectContent>
				{#each $palettes as palette (palette.name)}
					<SelectItem value={palette.name}>{palette.name}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
		<Badge variant="outline" class="gap-1">
			{#if isBuiltIn}<LockIcon /> Built-in{:else}Custom{/if}
		</Badge>
	</div>

	<div class="flex flex-wrap items-center gap-1 border border-border bg-background p-1">
		<Button size="xs" variant="ghost" onclick={selectAllRows}>Select all</Button>
		<Button size="xs" variant="ghost" onclick={deselectRows} disabled={selectedCount === 0}>
			Deselect
		</Button>
		<Button
			size="xs"
			variant="ghost"
			onclick={toggleSelectedVisibility}
			disabled={selectedCount === 0}
		>
			Toggle visibility
		</Button>
		<Button size="xs" variant="ghost" onclick={newPalette}>New</Button>
		<Button size="xs" variant="ghost" onclick={duplicatePalette}><CopyIcon /> Duplicate</Button>
		<Button size="xs" variant="ghost" onclick={() => importInput?.click()}
			><UploadIcon /> Import</Button
		>
		<Button size="xs" variant="ghost" onclick={() => exportPalette(false)}
			><DownloadIcon /> Export</Button
		>
		<div class="ml-auto flex items-center gap-1">
			<Button size="xs" variant="ghost" onclick={addColor} aria-label="Add color">
				<PlusIcon />
				<span class="hidden sm:inline">Add</span>
			</Button>
			<Button
				size="xs"
				variant="ghost"
				onclick={deleteSelectedColors}
				disabled={selectedCount === 0}
				aria-label="Delete selected colors"
			>
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

	{#if paletteMessage}
		<p
			role={paletteMessageTone === 'error' ? 'alert' : undefined}
			class="border px-2 py-1 text-xs {paletteMessageTone === 'error'
				? 'border-destructive/30 bg-destructive/10 text-destructive'
				: 'border-border bg-muted/50 text-muted-foreground'}"
		>
			{paletteMessage}
		</p>
	{/if}

	<div class="grid gap-3 border border-border bg-background p-3 text-sm">
		<div class="flex items-center gap-3">
			{@render swatchSquare({ name: 'Transparent', key: TRANSPARENT_KEY, kind: 'transparent' })}
			<div class="min-w-0 flex-1">
				<div class="font-medium">Transparent</div>
				<p class="text-xs text-muted-foreground">
					Every palette includes this swatch. Turn it off to map transparent pixels to visible
					colors.
				</p>
			</div>
			<VisibilityCheckbox
				checked={transparentEnabled}
				aria-label="Transparent swatch enabled"
				onCheckedChange={(next) =>
					setPaletteColorEnabled(TRANSPARENT_KEY, next, currentPalette.name)}
			/>
		</div>
		<div class="grid gap-2">
			<div class="grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2">
				<Label for="alpha-mode">Alpha</Label>
				<Select bind:value={alpha} type="single">
					<SelectTrigger id="alpha-mode">{alphaLabel}</SelectTrigger>
					<SelectContent>
						{#each ALPHA_MODES as mode (mode.id)}
							<SelectItem value={mode.id}>{mode.label}</SelectItem>
						{/each}
					</SelectContent>
				</Select>
			</div>
			{#if alpha === 'matte'}
				<div class="grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2">
					<Label for="matte-color">Matte</Label>
					<div class="flex gap-2">
						<Select bind:value={matteKey} type="single" disabled={!visiblePaletteColors.length}>
							<SelectTrigger id="matte-color" class="min-w-0 flex-1">{matteLabel}</SelectTrigger>
							<SelectContent>
								{#each visiblePaletteColors as color (color.key)}
									<SelectItem value={color.key}>{color.name} · {color.key}</SelectItem>
								{/each}
							</SelectContent>
						</Select>
						<Button
							type="button"
							variant="outline"
							onclick={useDarkestPaletteColor}
							disabled={!visiblePaletteColors.length}
						>
							Darkest
						</Button>
					</div>
				</div>
			{/if}
			<div class="grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2">
				<Label for="alpha-threshold">Threshold</Label>
				<div class="flex items-center gap-2">
					<Slider
						type="single"
						bind:value={alphaThreshold}
						min={0}
						max={255}
						step={1}
						disabled={alpha !== 'preserve'}
						aria-label="Alpha threshold"
					/>
					<span class="w-8 text-right text-xs text-muted-foreground tabular-nums"
						>{alphaThreshold}</span
					>
				</div>
			</div>
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
					{#each currentPalette.colors as color, i (color.key)}
						{@render row(color, i)}
					{/each}
				</tbody>
			</table>
		{:else}
			<div class="grid grid-cols-4 gap-1 p-2 sm:grid-cols-5 md:grid-cols-6 lg:grid-cols-7">
				{#each currentPalette.colors as color (color.key)}
					{@render gridSwatch(color)}
				{/each}
			</div>
		{/if}
	</ScrollArea>

	<div class="flex items-center justify-between text-xs text-muted-foreground">
		<span>{enabledCount} / {currentPalette.colors.length} colors active</span>
		<span>
			{selectedCount} selected · Enabled state persists locally.
			<button type="button" class="underline" onclick={() => exportPalette(true)}
				>Export all custom</button
			>
			{#if !isBuiltIn}
				· <button type="button" class="text-destructive underline" onclick={deletePalette}
					>Delete palette</button
				>
			{/if}
		</span>
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
	{@const isVisible = enabled(color)}
	{@const selected = selectedColorKeys[color.key] === true}
	<tr
		class="border-t border-border hover:bg-muted/40 data-selected:bg-muted/50"
		data-disabled={!isVisible || undefined}
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
				checked={isVisible}
				aria-label="Visible: {color.name}"
				onCheckedChange={(next) => setPaletteColorEnabled(color.key, next, currentPalette.name)}
			/>
		</td>
		<td class="p-1.5">{@render swatchSquare(color, 'sm')}</td>
		<td class="truncate p-1.5">{color.name}</td>
		<td class="hidden p-1.5 font-mono text-muted-foreground sm:table-cell">
			{color.kind === 'transparent' ? '—' : color.key}
		</td>
		<td class="hidden p-1.5 md:table-cell">{@render kindBadge(color)}</td>
		<td class="p-1.5">
			<DropdownMenu>
				<DropdownMenuTrigger aria-label="Row actions"><DotsIcon /></DropdownMenuTrigger>
				<DropdownMenuContent align="end">
					<DropdownMenuItem
						disabled={isBuiltIn || color.kind === 'transparent'}
						onclick={() => editColor(color)}
					>
						Edit…
					</DropdownMenuItem>
					<DropdownMenuItem
						disabled={isBuiltIn || color.kind === 'transparent'}
						onclick={() => duplicateColor(color)}
					>
						Duplicate color…
					</DropdownMenuItem>
					<DropdownMenuSeparator />
					<DropdownMenuItem
						disabled={isBuiltIn || color.kind === 'transparent'}
						onclick={() => deleteColor(color)}
					>
						Delete
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</td>
	</tr>
{/snippet}

{#snippet gridSwatch(color: PaletteColor)}
	{@const isVisible = enabled(color)}
	{@const selected = selectedColorKeys[color.key] === true}
	{@const reveal =
		'opacity-0 transition-opacity group-hover/swatch:opacity-100 group-focus-within/swatch:opacity-100'}
	{@const cornerBtn = `absolute z-10 inline-flex size-5 items-center justify-center bg-background/85 backdrop-blur-[1px] hover:bg-background focus-visible:ring-ring focus-visible:ring-1 focus-visible:outline-none disabled:cursor-not-allowed disabled:hover:bg-background/85 disabled:[&_svg]:opacity-40 [&_svg]:size-3 ${reveal}`}
	{@const cornerCheckbox = `absolute z-10 size-5 rounded-none bg-background/85 backdrop-blur-[1px] ${reveal}`}
	<figure
		class="group/swatch relative aspect-square overflow-hidden border border-border bg-muted data-selected:ring-2 data-selected:ring-primary data-disabled:opacity-40"
		data-disabled={!isVisible || undefined}
		data-selected={selected || undefined}
		aria-label="{color.name}{color.kind !== 'transparent' ? ` ${color.key}` : ''}{isVisible
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
			class={cornerBtn}
			disabled={isBuiltIn || color.kind === 'transparent'}
			aria-label="Edit {color.name}"
			title={isBuiltIn ? 'Built-in colors are immutable' : 'Edit color'}
			onclick={() => editColor(color)}><PencilIcon /></button
		>
		<button
			type="button"
			class="{cornerBtn} top-1 right-1 hover:text-destructive"
			disabled={isBuiltIn || color.kind === 'transparent'}
			aria-label="Delete {color.name}"
			title={isBuiltIn ? 'Built-in colors are immutable' : 'Delete color'}
			onclick={() => deleteColor(color)}><TrashIcon /></button
		>
		<Checkbox
			class="{cornerCheckbox} bottom-1 left-1"
			checked={selected}
			aria-label="Select {color.name}"
			onCheckedChange={(next) => setRowSelected(color.key, next)}
		/>
		<VisibilityCheckbox
			class="{cornerCheckbox} right-1 bottom-1"
			checked={isVisible}
			aria-label="Visible: {color.name}"
			onCheckedChange={(next) => setPaletteColorEnabled(color.key, next, currentPalette.name)}
		/>
		<figcaption class="sr-only">
			{color.name}{color.kind !== 'transparent' ? ` ${color.key}` : ''} · {isVisible
				? 'visible'
				: 'hidden'}
		</figcaption>
	</figure>
{/snippet}
