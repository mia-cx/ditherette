<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { Slider } from '$lib/components/ui/slider';
	import { Label } from '$lib/components/ui/label';
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogFooter,
		DialogHeader,
		DialogTitle
	} from '$lib/components/ui/dialog';
	import { paletteEnabledKey } from '$lib/palette/wplace';
	import { cn } from '$lib/utils';
	import {
		activePalette,
		activePaletteName,
		addColorToActivePalette,
		createCustomPalette,
		customPalettes,
		deleteCustomPalette,
		deleteActivePaletteColors,
		duplicateActivePalette,
		duplicateActivePaletteColor,
		editActivePaletteColor,
		importCustomPaletteData,
		outputSettings,
		paletteEnabled,
		previewCustomPaletteImport,
		palettes,
		selectedPalette,
		setPaletteColorEnabled,
		updateOutputSettings
	} from '$lib/stores/app';
	import type { Palette, PaletteColor } from '$lib/processing/types';
	import { ALPHA_MODES } from './output-options';
	import CopyIcon from 'phosphor-svelte/lib/Copy';
	import PencilIcon from 'phosphor-svelte/lib/PencilSimple';
	import PlusIcon from 'phosphor-svelte/lib/Plus';
	import TrashIcon from 'phosphor-svelte/lib/Trash';
	import VisibilityCheckbox from './VisibilityCheckbox.svelte';
	import ColorEditDialog from './ColorEditDialog.svelte';
	import PaletteToolbar from './PaletteToolbar.svelte';

	type Props = { fillHeight?: boolean };
	let { fillHeight = false }: Props = $props();

	// Bound select state is intentionally local; effects synchronize it with the persistent nanostore.
	// eslint-disable-next-line svelte/prefer-writable-derived
	let preset = $state(activePaletteName.get());
	let selectedColorKeys = $state<Record<string, boolean>>({});
	let selectedTagKeys = $state<Record<string, boolean>>({});
	let importInput = $state<HTMLInputElement>();
	let paletteMessage = $state<string>();
	let paletteMessageTone = $state<'neutral' | 'error'>('neutral');
	let colorDialogOpen = $state(false);
	let colorDialogMode = $state<'add' | 'edit' | 'duplicate'>('add');
	let colorDialogColor = $state<PaletteColor>();
	let colorName = $state('');
	let colorHex = $state('#FF00AA');
	let colorTags = $state<string[]>([]);
	let transparentDialogOpen = $state(false);
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

	const MAX_PALETTE_IMPORT_BYTES = 512 * 1024;

	const currentPalette = $derived($activePalette);
	const isBuiltIn = $derived(currentPalette.source === 'wplace');
	const selectedCount = $derived(Object.values(selectedColorKeys).filter(Boolean).length);
	const enabledCount = $derived(
		currentPalette.colors.filter(
			(color) => $paletteEnabled[paletteEnabledKey(currentPalette.name, color.key)] !== false
		).length
	);
	const allRowsSelected = $derived(
		currentPalette.colors.length > 0 && selectedCount === currentPalette.colors.length
	);
	const someRowsSelected = $derived(selectedCount > 0 && !allRowsSelected);
	const alphaLabel = $derived(ALPHA_MODES.find((mode) => mode.id === alpha)?.label ?? 'Alpha');
	const visiblePaletteColors = $derived($selectedPalette.filter((color) => color.rgb));
	const matteLabel = $derived(
		visiblePaletteColors.find((color) => color.key === matteKey)?.name ?? 'Select matte color'
	);

	type TagSelectionSource = 'kind' | 'tag';
	type TagSelection = { source: TagSelectionSource; tag: string };
	const tagKeySeparator = '\u0000';

	function tagSelectionKey(source: TagSelectionSource, tag: string) {
		return `${source}${tagKeySeparator}${tag}`;
	}

	function parseTagSelectionKey(key: string): TagSelection {
		const separatorIndex = key.indexOf(tagKeySeparator);
		return {
			source: key.slice(0, separatorIndex) as TagSelectionSource,
			tag: key.slice(separatorIndex + tagKeySeparator.length)
		};
	}

	function colorHasPaletteTag(color: PaletteColor, source: TagSelectionSource, tag: string) {
		return source === 'kind' ? color.kind === tag : color.tags?.includes(tag) === true;
	}

	function tagColorKeys(source: TagSelectionSource, tag: string) {
		return currentPalette.colors
			.filter((color) => colorHasPaletteTag(color, source, tag))
			.map((color) => color.key);
	}

	function selectedTagSelections(tagKeys = selectedTagKeys) {
		return Object.entries(tagKeys)
			.filter(([, selected]) => selected)
			.map(([key]) => parseTagSelectionKey(key));
	}

	function colorMatchesSelectedTag(color: PaletteColor, tagKeys = selectedTagKeys) {
		return selectedTagSelections(tagKeys).some(({ source, tag }) =>
			colorHasPaletteTag(color, source, tag)
		);
	}

	function tagIsSelected(source: TagSelectionSource, tag: string) {
		return selectedTagKeys[tagSelectionKey(source, tag)] === true;
	}

	function pruneSelectedTags(nextSelectedColorKeys: Record<string, boolean>) {
		selectedTagKeys = Object.fromEntries(
			Object.entries(selectedTagKeys).filter(([key, selected]) => {
				if (!selected) return false;
				const { source, tag } = parseTagSelectionKey(key);
				const keys = tagColorKeys(source, tag);
				return (
					keys.length > 0 && keys.every((colorKey) => nextSelectedColorKeys[colorKey] === true)
				);
			})
		);
	}

	function toggleTagSelection(source: TagSelectionSource, tag: string) {
		const keys = tagColorKeys(source, tag);
		if (!keys.length) return;
		const key = tagSelectionKey(source, tag);
		const nextTagKeys = { ...selectedTagKeys };
		const nextSelectedColorKeys = { ...selectedColorKeys };

		if (nextTagKeys[key]) {
			delete nextTagKeys[key];
			for (const color of currentPalette.colors) {
				if (!colorHasPaletteTag(color, source, tag)) continue;
				if (colorMatchesSelectedTag(color, nextTagKeys)) continue;
				delete nextSelectedColorKeys[color.key];
			}
		} else {
			nextTagKeys[key] = true;
			for (const colorKey of keys) nextSelectedColorKeys[colorKey] = true;
		}

		selectedTagKeys = nextTagKeys;
		selectedColorKeys = nextSelectedColorKeys;
	}

	function tagButtonClass(selected: boolean, variant: 'default' | 'outline' | 'secondary') {
		return cn(
			'inline-flex h-5 w-fit shrink-0 cursor-pointer items-center justify-center overflow-hidden rounded-none border px-2 py-0.5 text-xs font-medium whitespace-nowrap transition-colors focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring/50',
			selected && 'border-primary bg-primary text-primary-foreground hover:bg-primary/90',
			!selected &&
				variant === 'default' &&
				'border-primary bg-primary text-primary-foreground hover:bg-primary/80',
			!selected &&
				variant === 'secondary' &&
				'border-border bg-secondary text-secondary-foreground hover:bg-secondary/80',
			!selected && variant === 'outline' && 'border-border text-foreground hover:bg-muted'
		);
	}

	$effect(() => {
		activePaletteName.set(preset);
		selectedColorKeys = {};
		selectedTagKeys = {};
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
		if (!visiblePaletteColors.length) return;
		if (visiblePaletteColors.some((color) => color.key === matteKey)) return;
		matteKey = fallbackMatteKey(matteKey);
	});

	$effect(() => {
		updateOutputSettings({ alphaMode: alpha, alphaThreshold, matteKey });
	});

	function enabled(color: PaletteColor, palette: Palette = currentPalette) {
		return $paletteEnabled[paletteEnabledKey(palette.name, color.key)] !== false;
	}

	function setRowSelected(key: string, selected: boolean) {
		const nextSelectedColorKeys = { ...selectedColorKeys };
		if (selected) {
			nextSelectedColorKeys[key] = true;
		} else {
			delete nextSelectedColorKeys[key];
		}
		selectedColorKeys = nextSelectedColorKeys;
		pruneSelectedTags(nextSelectedColorKeys);
	}

	function reconcileEditedColorSelection(previousKey: string, editedColor: PaletteColor) {
		const nextSelectedColorKeys = { ...selectedColorKeys };
		if (previousKey !== editedColor.key && nextSelectedColorKeys[previousKey]) {
			delete nextSelectedColorKeys[previousKey];
			nextSelectedColorKeys[editedColor.key] = true;
		}
		selectedColorKeys = nextSelectedColorKeys;
		pruneSelectedTags(nextSelectedColorKeys);
	}

	function selectAllRows() {
		selectedColorKeys = Object.fromEntries(currentPalette.colors.map((color) => [color.key, true]));
		selectedTagKeys = {};
	}

	function setAllRowsSelected(selected: boolean) {
		if (selected) {
			selectAllRows();
			return;
		}
		deselectRows();
	}

	function deselectRows() {
		selectedColorKeys = {};
		selectedTagKeys = {};
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

	function fallbackMatteKey(previousKey: string) {
		const previous = currentPalette.colors.find((color) => color.key === previousKey)?.rgb;
		if (!previous) return visiblePaletteColors[0]!.key;
		let best = visiblePaletteColors[0]!;
		let bestDistance = Number.POSITIVE_INFINITY;
		for (const color of visiblePaletteColors) {
			const rgb = color.rgb!;
			const distance =
				(rgb.r - previous.r) ** 2 + (rgb.g - previous.g) ** 2 + (rgb.b - previous.b) ** 2;
			if (distance < bestDistance) {
				best = color;
				bestDistance = distance;
			}
		}
		return best.key;
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
			return true;
		} catch (error) {
			setPaletteMessage(error instanceof Error ? error.message : 'Palette action failed.', 'error');
			return false;
		}
	}

	function newPalette() {
		const name = prompt('Custom palette name', 'My palette');
		if (!name) return;
		withPaletteError(() => createCustomPalette(name));
	}

	function duplicatePalette(palette = currentPalette) {
		const name = prompt('Duplicate palette as', `${palette.name} Copy`);
		if (!name) return;
		activePaletteName.set(palette.name);
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
		colorTags = color?.tags ? [...color.tags] : [];
		colorDialogOpen = true;
	}

	function saveColorDialog() {
		let editedColor: PaletteColor | undefined;
		const previousKey = colorDialogColor?.key;
		const saved = withPaletteError(() => {
			if (colorDialogMode === 'edit' && colorDialogColor) {
				editedColor = editActivePaletteColor(colorDialogColor.key, colorName, colorHex, colorTags);
			} else if (colorDialogMode === 'duplicate' && colorDialogColor) {
				duplicateActivePaletteColor(colorDialogColor.key, colorName, colorHex, colorTags);
			} else {
				addColorToActivePalette(colorName, colorHex, colorTags);
			}
		});
		if (!saved) return;
		if (colorDialogMode === 'edit' && previousKey && editedColor) {
			reconcileEditedColorSelection(previousKey, editedColor);
		}
		colorDialogOpen = false;
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
		if (color.kind === 'transparent') {
			transparentDialogOpen = true;
			return;
		}
		openColorDialog('edit', color);
	}

	function duplicateColor(color: PaletteColor) {
		openColorDialog('duplicate', color);
	}

	function duplicateSelectedColors() {
		if (isBuiltIn) {
			setPaletteMessage(
				'Built-in palettes are immutable. Duplicate Wplace before duplicating colors.',
				'error'
			);
			return;
		}
		const colors = currentPalette.colors.filter(
			(color) => selectedColorKeys[color.key] && color.kind !== 'transparent'
		);
		if (colors.length !== 1) {
			setPaletteMessage(
				'Select one custom color to duplicate. Bulk duplicate needs the edit flow.'
			);
			return;
		}
		duplicateColor(colors[0]);
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
		const deleted = withPaletteError(() => deleteActivePaletteColors([key]));
		if (!deleted) return;
		const nextSelectedColorKeys = { ...selectedColorKeys };
		delete nextSelectedColorKeys[key];
		selectedColorKeys = nextSelectedColorKeys;
		pruneSelectedTags(nextSelectedColorKeys);
		deleteDialogOpen = false;
		deleteColorKey = undefined;
	}

	function deletePalette(palette = currentPalette) {
		if (palette.source === 'wplace') {
			setPaletteMessage('The Wplace palette cannot be deleted.', 'error');
			return;
		}
		if (!confirm(`Delete custom palette ${palette.name}?`)) return;
		withPaletteError(() => deleteCustomPalette(palette.name));
	}

	function exportActivePalette() {
		exportPalette({ scope: 'active' });
	}

	function exportPalette({ scope }: { scope: 'active' | 'all' }) {
		const data =
			scope === 'all'
				? $palettes.filter((palette) => palette.source === 'custom')
				: [currentPalette].filter((palette) => palette.source === 'custom');
		if (!data.length) {
			setPaletteMessage('There are no custom palettes to export.', 'error');
			return;
		}
		const blob = new Blob([JSON.stringify(scope === 'all' ? data : data[0], null, 2)], {
			type: 'application/json'
		});
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = scope === 'all' ? 'ditherette-palettes.json' : `${data[0].name}.palette.json`;
		link.click();
		URL.revokeObjectURL(url);
	}

	async function importPalette(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		try {
			if (file.size > MAX_PALETTE_IMPORT_BYTES) {
				throw new Error(
					`Palette imports must be ${Math.floor(MAX_PALETTE_IMPORT_BYTES / 1024)} KB or smaller.`
				);
			}
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

<ColorEditDialog
	bind:open={colorDialogOpen}
	bind:name={colorName}
	bind:hex={colorHex}
	bind:tags={colorTags}
	mode={colorDialogMode}
	onSave={saveColorDialog}
/>

<Dialog bind:open={transparentDialogOpen}>
	<DialogContent>
		<DialogHeader>
			<DialogTitle>Transparent swatch settings</DialogTitle>
			<DialogDescription>
				Transparent is fixed in every palette. Use these controls to decide how source alpha is
				handled during processing.
			</DialogDescription>
		</DialogHeader>
		<div class="grid gap-3 py-2 text-sm">
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
		<DialogFooter>
			<Button onclick={() => (transparentDialogOpen = false)}>Done</Button>
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

<section class={fillHeight ? 'flex h-full flex-col' : 'flex flex-col'} aria-label="Palette editor">
	<ScrollArea class="min-h-0 flex-1">
		<div class="flex min-h-full flex-col gap-2">
			<PaletteToolbar
				bind:preset
				palettes={$palettes}
				{currentPalette}
				onNewPalette={newPalette}
				onDuplicatePalette={duplicatePalette}
				onDeletePalette={deletePalette}
				onImportPalette={() => importInput?.click()}
				onExportPalette={exportActivePalette}
			/>

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

			<div class="flex min-h-0 flex-1 flex-col border border-border bg-background">
				<div
					class="sticky top-0 z-20 flex min-h-9 flex-wrap items-center gap-2 border-b border-border bg-background/95 px-2 py-1.5 backdrop-blur"
				>
					<div class="flex min-w-0 items-center gap-2">
						<Checkbox
							checked={allRowsSelected}
							indeterminate={someRowsSelected}
							aria-label="Select all palette colors"
							onCheckedChange={(next) => setAllRowsSelected(next)}
						/>
						{#if selectedCount > 0}
							<span class="text-xs text-muted-foreground">{selectedCount} selected</span>
						{/if}
					</div>
					<div class="ml-auto flex items-center gap-1">
						<Button
							size="xs"
							variant="ghost"
							onclick={toggleSelectedVisibility}
							disabled={selectedCount === 0}
						>
							Toggle visibility
						</Button>
						<Button
							size="icon-xs"
							variant="ghost"
							onclick={duplicateSelectedColors}
							disabled={selectedCount === 0 || isBuiltIn}
							aria-label="Duplicate selected colors"
						>
							<CopyIcon weight="bold" />
						</Button>
						<Button
							size="icon-xs"
							variant="ghost"
							class="hover:text-destructive"
							onclick={deleteSelectedColors}
							disabled={selectedCount === 0 || isBuiltIn}
							aria-label="Delete selected colors"
						>
							<TrashIcon weight="bold" />
						</Button>
					</div>
				</div>
				<table class="w-full border-collapse text-xs">
					<thead class="sticky top-9 z-10 border-b border-border bg-muted/80 backdrop-blur">
						<tr class="text-muted-foreground">
							<th class="w-8 p-1.5"><span class="sr-only">Selected</span></th>
							<th class="w-8 p-1.5"><span class="sr-only">Swatch</span></th>
							<th class="w-8 p-1.5"><span class="sr-only">Visibility</span></th>
							<th class="p-1.5 text-left font-medium">Name</th>
							<th class="p-1.5 text-left font-medium">Hex</th>
							<th class="p-1.5 text-left font-medium">Tags</th>
							<th class="p-1.5 text-right font-medium">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each currentPalette.colors as color (color.key)}
							{@render row(color)}
						{/each}
					</tbody>
				</table>

				<div
					class="sticky bottom-0 z-20 mt-auto flex items-center gap-3 border-t border-border bg-background/95 px-2 py-1.5 text-xs text-muted-foreground backdrop-blur"
				>
					<Button size="xs" variant="outline" onclick={addColor}>
						<PlusIcon weight="bold" />
						New
					</Button>
					<span class="ml-auto tabular-nums"
						>{enabledCount}/{currentPalette.colors.length} active</span
					>
				</div>
			</div>
		</div>
	</ScrollArea>
</section>

{#snippet swatchSquare(color: PaletteColor)}
	{#if color.kind === 'transparent'}
		<span
			class="block size-5 border border-border [background-image:repeating-conic-gradient(rgba(0,0,0,0.3)_0%_25%,transparent_0%_50%)] [background-size:8px_8px]"
			aria-label="Transparent"
		></span>
	{:else}
		<span
			class="block size-5 border border-border"
			style="background-color: {color.key}"
			aria-hidden="true"
		></span>
	{/if}
{/snippet}

{#snippet tagButton(
	source: TagSelectionSource,
	tag: string,
	label: string,
	variant: 'default' | 'outline' | 'secondary' = 'outline'
)}
	{@const selected = tagIsSelected(source, tag)}
	<button
		type="button"
		class={tagButtonClass(selected, variant)}
		aria-pressed={selected}
		aria-label="{selected ? 'Deselect' : 'Select'} all {label} colors"
		onclick={() => toggleTagSelection(source, tag)}
	>
		{label}
	</button>
{/snippet}

{#snippet kindBadge(color: PaletteColor)}
	<div class="flex flex-wrap gap-1">
		{#if color.kind === 'transparent'}
			{@render tagButton('kind', color.kind, 'Transparent')}
		{:else if color.kind === 'premium'}
			{@render tagButton('kind', color.kind, 'Premium', 'secondary')}
		{:else if color.kind === 'custom'}
			{@render tagButton('kind', color.kind, 'Custom', 'default')}
		{:else}
			{@render tagButton('kind', color.kind, 'Free')}
		{/if}
		{#each color.tags ?? [] as tag (tag)}
			{@render tagButton('tag', tag, tag)}
		{/each}
	</div>
{/snippet}

{#snippet row(color: PaletteColor)}
	{@const isVisible = enabled(color)}
	{@const selected = selectedColorKeys[color.key] === true}
	{@const editable = color.kind === 'transparent' || !isBuiltIn}
	{@const mutable = !isBuiltIn && color.kind !== 'transparent'}
	<tr
		class="border-t border-border hover:bg-muted/40 data-selected:bg-muted/50 data-disabled:opacity-55"
		data-disabled={!isVisible || undefined}
		data-selected={selected || undefined}
	>
		<td class="p-1.5 align-middle">
			<Checkbox
				checked={selected}
				aria-label="Select {color.name}"
				onCheckedChange={(next) => setRowSelected(color.key, next)}
			/>
		</td>
		<td class="p-1.5 align-middle">{@render swatchSquare(color)}</td>
		<td class="p-1.5 align-middle">
			<VisibilityCheckbox
				checked={isVisible}
				aria-label="Visible: {color.name}"
				onCheckedChange={(next) =>
					setPaletteColorEnabled(color.key, next === true, currentPalette.name)}
			/>
		</td>
		<td class="max-w-32 truncate p-1.5 align-middle font-medium">{color.name}</td>
		<td class="p-1.5 align-middle font-mono text-muted-foreground">
			{color.kind === 'transparent' ? '—' : color.key}
		</td>
		<td class="p-1.5 align-middle">{@render kindBadge(color)}</td>
		<td class="p-1.5 align-middle">
			<div class="flex justify-end gap-1">
				<Button size="xs" variant="ghost" disabled={!editable} onclick={() => editColor(color)}>
					<PencilIcon weight="bold" />
					Edit
				</Button>
				<Button
					size="icon-xs"
					variant="ghost"
					disabled={!mutable}
					aria-label="Duplicate {color.name}"
					onclick={() => duplicateColor(color)}
				>
					<CopyIcon weight="bold" />
				</Button>
				<Button
					size="icon-xs"
					variant="ghost"
					class="hover:text-destructive"
					disabled={!mutable}
					aria-label="Delete {color.name}"
					onclick={() => deleteColor(color)}
				>
					<TrashIcon weight="bold" />
				</Button>
			</div>
		</td>
	</tr>
{/snippet}
