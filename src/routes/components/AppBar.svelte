<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import {
		Menubar,
		MenubarCheckboxItem,
		MenubarContent,
		MenubarItem,
		MenubarMenu,
		MenubarRadioGroup,
		MenubarRadioItem,
		MenubarSeparator,
		MenubarShortcut,
		MenubarSub,
		MenubarSubContent,
		MenubarSubTrigger,
		MenubarTrigger
	} from '$lib/components/ui/menubar';
	import { EFFECTS, EFFECT_KINDS, MAX_EFFECT_LAYERS, type EffectKind } from '$lib/effects/catalog';
	import { exportable, exportPng } from '$lib/processing/export';
	import {
		colorSpace,
		ditherSettings,
		outputSettings,
		previewSettings,
		updateDitherSettings,
		updateOutputSettings,
		updatePreviewSettings,
		type PreviewMode
	} from '$lib/stores/app';
	import { addEffect, effectLayers } from '$lib/stores/effects';
	import { loadThemeChoice, setThemeChoice, themeChoice, type ThemeChoice } from '$lib/theme';
	import { COLOR_SPACES } from './color-space-options';
	import { DITHER_ALGORITHMS } from './dither-options';
	import { RESIZE_MODES } from './output-options';
	import { cropping, previewCommands } from './preview-commands';
	import {
		WINDOWS,
		openEffectWindow,
		openWindows,
		resetLayout,
		showWindow,
		toggleWindow
	} from './studio/workspace';

	type Props = {
		hasImage: boolean;
		/** Docked windows are available: adds the Window menu and window commands. */
		studio: boolean;
		extras?: Snippet;
		onChooseImage: () => void;
		onClear: () => void | Promise<void>;
	};
	let { hasImage, studio, extras, onChooseImage, onClear }: Props = $props();

	let mac = $state(true);
	onMount(() => {
		mac = /Mac|iPhone|iPad/.test(navigator.userAgent);
		loadThemeChoice();
	});

	const PREVIEW_MODES = [
		{ id: 'side-by-side', label: 'Side by side' },
		{ id: 'ab-reveal', label: 'A/B reveal' }
	] as const satisfies readonly { id: PreviewMode; label: string }[];
	const THEMES = [
		{ id: 'system', label: 'System' },
		{ id: 'light', label: 'Light' },
		{ id: 'dark', label: 'Dark' }
	] as const satisfies readonly { id: ThemeChoice; label: string }[];

	/** The option a radio group reported, typed by its list. */
	function pick<T extends string>(options: readonly { id: T }[], value: string) {
		return options.find((option) => option.id === value)?.id;
	}

	const command = (letter: string) => (mac ? `⌘${letter}` : `Ctrl+${letter}`);

	/** Commands with the modifier, and bare view keys that leave browser zoom alone. */
	const MODIFIED: Record<string, () => void> = { o: () => onChooseImage(), e: exportPng };
	const BARE: Record<string, () => void> = {
		'+': () => $previewCommands?.zoomIn(),
		'=': () => $previewCommands?.zoomIn(),
		'-': () => $previewCommands?.zoomOut(),
		'0': () => $previewCommands?.fit(),
		'1': () => $previewCommands?.actualSize()
	};

	function shortcut(event: KeyboardEvent) {
		const target = event.target as HTMLElement | null;
		if (event.altKey || target?.closest('input, textarea, select, [contenteditable="true"]'))
			return;
		const modified = mac ? event.metaKey : event.ctrlKey;
		const other = mac ? event.ctrlKey : event.metaKey;
		if (other) return;
		const run = modified
			? MODIFIED[event.key.toLowerCase()]
			: hasImage && !event.shiftKey
				? BARE[event.key]
				: undefined;
		if (!run) return;
		event.preventDefault();
		run();
	}

	function adjust(kind: EffectKind) {
		const layer = addEffect(kind);
		if (studio) openEffectWindow(layer.id);
	}
</script>

<svelte:window onkeydown={shortcut} />

<header
	class="sticky top-0 z-30 flex h-10 w-full items-center gap-3 border-b border-border bg-background px-3"
>
	<a href={resolve('/')} class="text-sm font-semibold tracking-tight">ditherette</a>

	<Menubar class="h-full border-0 bg-transparent p-0">
		<MenubarMenu>
			<MenubarTrigger class="px-2 text-sm font-normal">File</MenubarTrigger>
			<MenubarContent align="start" class="min-w-52">
				<MenubarItem onSelect={onChooseImage}>
					Open image…<MenubarShortcut>{command('O')}</MenubarShortcut>
				</MenubarItem>
				<MenubarItem disabled={!$exportable} onSelect={exportPng}>
					Export PNG<MenubarShortcut>{command('E')}</MenubarShortcut>
				</MenubarItem>
				<MenubarSeparator />
				<MenubarItem variant="destructive" disabled={!hasImage} onSelect={onClear}>
					Clear image
				</MenubarItem>
			</MenubarContent>
		</MenubarMenu>

		<MenubarMenu>
			<MenubarTrigger class="px-2 text-sm font-normal">Edit</MenubarTrigger>
			<MenubarContent align="start" class="min-w-52">
				<MenubarCheckboxItem
					checked={$cropping}
					disabled={!hasImage || !$previewCommands}
					onCheckedChange={() => $previewCommands?.toggleCrop()}>Crop</MenubarCheckboxItem
				>
				<MenubarItem
					disabled={!$outputSettings.crop}
					onSelect={() => updateOutputSettings({ crop: undefined })}>Clear crop</MenubarItem
				>
			</MenubarContent>
		</MenubarMenu>

		<MenubarMenu>
			<MenubarTrigger class="px-2 text-sm font-normal">Image</MenubarTrigger>
			<MenubarContent align="start" class="min-w-52">
				<MenubarSub>
					<MenubarSubTrigger>Adjustments</MenubarSubTrigger>
					<MenubarSubContent class="min-w-52">
						{#each EFFECT_KINDS as kind (kind)}
							<MenubarItem
								disabled={$effectLayers.length >= MAX_EFFECT_LAYERS}
								onSelect={() => adjust(kind)}>{EFFECTS[kind].label}</MenubarItem
							>
						{/each}
					</MenubarSubContent>
				</MenubarSub>
				{#if studio}
					<MenubarItem onSelect={() => showWindow('dimensions')}>Image size…</MenubarItem>
				{/if}
				<MenubarSeparator />
				<MenubarSub>
					<MenubarSubTrigger>Resample</MenubarSubTrigger>
					<MenubarSubContent class="min-w-52">
						<MenubarRadioGroup
							value={$outputSettings.resize}
							onValueChange={(value) => {
								const resize = pick(RESIZE_MODES, value);
								if (resize) updateOutputSettings({ resize });
							}}
						>
							{#each RESIZE_MODES as mode (mode.id)}
								<MenubarRadioItem value={mode.id}>{mode.label}</MenubarRadioItem>
							{/each}
						</MenubarRadioGroup>
					</MenubarSubContent>
				</MenubarSub>
				<MenubarSub>
					<MenubarSubTrigger>Dither</MenubarSubTrigger>
					<MenubarSubContent class="min-w-52">
						<MenubarRadioGroup
							value={$ditherSettings.algorithm}
							onValueChange={(value) => {
								const algorithm = pick(DITHER_ALGORITHMS, value);
								if (algorithm) updateDitherSettings({ algorithm });
							}}
						>
							{#each DITHER_ALGORITHMS as algorithm (algorithm.id)}
								<MenubarRadioItem value={algorithm.id}>{algorithm.label}</MenubarRadioItem>
							{/each}
						</MenubarRadioGroup>
					</MenubarSubContent>
				</MenubarSub>
				<MenubarSub>
					<MenubarSubTrigger>Color space</MenubarSubTrigger>
					<MenubarSubContent class="min-w-52">
						<MenubarRadioGroup
							value={$colorSpace}
							onValueChange={(value) => {
								const space = pick(COLOR_SPACES, value);
								if (space) colorSpace.set(space);
							}}
						>
							{#each COLOR_SPACES as space (space.id)}
								<MenubarRadioItem value={space.id}>{space.label}</MenubarRadioItem>
							{/each}
						</MenubarRadioGroup>
					</MenubarSubContent>
				</MenubarSub>
			</MenubarContent>
		</MenubarMenu>

		<MenubarMenu>
			<MenubarTrigger class="px-2 text-sm font-normal">View</MenubarTrigger>
			<MenubarContent align="start" class="min-w-52">
				<MenubarRadioGroup
					value={$previewSettings.mode ?? (studio ? 'side-by-side' : 'ab-reveal')}
					onValueChange={(value) => {
						const mode = pick(PREVIEW_MODES, value);
						if (mode) updatePreviewSettings({ mode });
					}}
				>
					{#each PREVIEW_MODES as mode (mode.id)}
						<MenubarRadioItem value={mode.id}>{mode.label}</MenubarRadioItem>
					{/each}
				</MenubarRadioGroup>
				<MenubarSeparator />
				<MenubarItem
					disabled={!hasImage || !$previewCommands}
					onSelect={() => $previewCommands?.zoomIn()}
				>
					Zoom in<MenubarShortcut>+</MenubarShortcut>
				</MenubarItem>
				<MenubarItem
					disabled={!hasImage || !$previewCommands}
					onSelect={() => $previewCommands?.zoomOut()}
				>
					Zoom out<MenubarShortcut>−</MenubarShortcut>
				</MenubarItem>
				<MenubarItem
					disabled={!hasImage || !$previewCommands}
					onSelect={() => $previewCommands?.fit()}
				>
					Fit to window<MenubarShortcut>0</MenubarShortcut>
				</MenubarItem>
				<MenubarItem
					disabled={!hasImage || !$previewCommands}
					onSelect={() => $previewCommands?.actualSize()}
				>
					Actual size<MenubarShortcut>1</MenubarShortcut>
				</MenubarItem>
				<MenubarSeparator />
				<MenubarSub>
					<MenubarSubTrigger>Theme</MenubarSubTrigger>
					<MenubarSubContent class="min-w-40">
						<MenubarRadioGroup
							value={$themeChoice}
							onValueChange={(value) => {
								const choice = pick(THEMES, value);
								if (choice) setThemeChoice(choice);
							}}
						>
							{#each THEMES as theme (theme.id)}
								<MenubarRadioItem value={theme.id}>{theme.label}</MenubarRadioItem>
							{/each}
						</MenubarRadioGroup>
					</MenubarSubContent>
				</MenubarSub>
			</MenubarContent>
		</MenubarMenu>

		{#if studio}
			<MenubarMenu>
				<MenubarTrigger class="px-2 text-sm font-normal">Window</MenubarTrigger>
				<MenubarContent align="start" class="min-w-52">
					{#each WINDOWS as window (window.id)}
						<MenubarCheckboxItem
							checked={$openWindows.has(window.id)}
							onCheckedChange={() => toggleWindow(window.id)}>{window.title}</MenubarCheckboxItem
						>
					{/each}
					<MenubarSeparator />
					<MenubarItem onSelect={resetLayout}>Reset layout</MenubarItem>
				</MenubarContent>
			</MenubarMenu>
		{/if}
	</Menubar>

	{#if extras}
		<div class="ml-auto flex items-center gap-1.5">
			{@render extras()}
		</div>
	{/if}
</header>
