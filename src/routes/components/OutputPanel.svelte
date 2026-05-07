<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import { Slider } from '$lib/components/ui/slider';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import { RESIZE_MODES, ALPHA_MODES } from './sample-data';
	import { outputSettings, sourceMeta, updateOutputSettings } from '$lib/stores/app';
	import { clampOutputSize } from '$lib/processing/types';
	import LinkIcon from 'phosphor-svelte/lib/Link';
	import LinkBreakIcon from 'phosphor-svelte/lib/LinkBreak';

	type Props = {
		hasImage?: boolean;
		hideHeading?: boolean;
	};

	let { hasImage = false, hideHeading = false }: Props = $props();

	const initial = outputSettings.get();
	let width = $state<number>(initial.width);
	let height = $state<number>(initial.height);
	let lockAspect = $state(initial.lockAspect);
	let fit = $state(initial.fit);
	let resize = $state(initial.resize);
	let alpha = $state(initial.alphaMode);
	let alphaThreshold = $state<number>(initial.alphaThreshold);

	const resizeLabel = $derived(RESIZE_MODES.find((r) => r.id === resize)?.label ?? 'Resize');
	const alphaLabel = $derived(ALPHA_MODES.find((a) => a.id === alpha)?.label ?? 'Alpha');

	$effect(() =>
		outputSettings.subscribe((settings) => {
			width = settings.width;
			height = settings.height;
			lockAspect = settings.lockAspect;
			fit = settings.fit;
			resize = settings.resize;
			alpha = settings.alphaMode;
			alphaThreshold = settings.alphaThreshold;
		})
	);

	$effect(() => {
		const clamped = clampOutputSize(width, height);
		updateOutputSettings({
			width: clamped.width,
			height: clamped.height,
			lockAspect,
			fit,
			resize,
			alphaMode: alpha,
			alphaThreshold
		});
	});

	function setWidth(value: number) {
		width = Math.max(1, Math.round(value || 1));
		if (lockAspect && $sourceMeta)
			height = Math.max(1, Math.round((width * $sourceMeta.height) / $sourceMeta.width));
	}

	function setHeight(value: number) {
		height = Math.max(1, Math.round(value || 1));
		if (lockAspect && $sourceMeta)
			width = Math.max(1, Math.round((height * $sourceMeta.width) / $sourceMeta.height));
	}

	function resetDimensionsToCrop() {
		const crop = $outputSettings.crop;
		if (!crop) return;
		width = Math.max(1, Math.round(crop.width));
		height = Math.max(1, Math.round(crop.height));
		updateOutputSettings({ width, height });
	}

	function clearCrop() {
		updateOutputSettings({ crop: undefined });
	}
</script>

<section class="flex flex-col gap-4" aria-label="Output controls">
	{#if !hideHeading}
		<div class="flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold tracking-tight">Output</h2>
			<p class="text-xs text-muted-foreground">Size, resize, and alpha.</p>
		</div>
	{/if}

	<div class="grid gap-1.5">
		<Label class="text-xs tracking-wide text-muted-foreground uppercase">Dimensions</Label>
		<div class="flex items-center gap-1.5">
			<div class="grid flex-1 gap-1">
				<Label for="out-width" class="text-xs">Width</Label>
				<Input
					id="out-width"
					type="number"
					inputmode="numeric"
					min="1"
					max="16384"
					step="1"
					value={width}
					disabled={!hasImage}
					oninput={(event) => setWidth(Number((event.currentTarget as HTMLInputElement).value))}
				/>
			</div>
			<Button
				variant="outline"
				size="icon-sm"
				class="mt-5"
				aria-label={lockAspect ? 'Aspect ratio locked' : 'Aspect ratio unlocked'}
				onclick={() => (lockAspect = !lockAspect)}
				aria-pressed={lockAspect}
			>
				{#if lockAspect}<LinkIcon />{:else}<LinkBreakIcon />{/if}
			</Button>
			<div class="grid flex-1 gap-1">
				<Label for="out-height" class="text-xs">Height</Label>
				<Input
					id="out-height"
					type="number"
					inputmode="numeric"
					min="1"
					max="16384"
					step="1"
					value={height}
					disabled={!hasImage}
					oninput={(event) => setHeight(Number((event.currentTarget as HTMLInputElement).value))}
				/>
			</div>
		</div>
		{#if $outputSettings.crop}
			<div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
				<span class="font-mono">
					Crop {$outputSettings.crop.width.toFixed(0)}×{$outputSettings.crop.height.toFixed(0)}
				</span>
				<Button size="xs" variant="outline" onclick={resetDimensionsToCrop}
					>Reset dimensions to crop</Button
				>
				<Button size="xs" variant="ghost" onclick={clearCrop}>Clear crop</Button>
			</div>
		{/if}
	</div>

	<div class="grid gap-1.5">
		<Label for="fit-mode">Fit mode</Label>
		<ToggleGroup type="single" bind:value={fit} variant="outline" class="w-full">
			<ToggleGroupItem value="stretch" class="flex-1">Stretch</ToggleGroupItem>
			<ToggleGroupItem value="contain" class="flex-1">Contain</ToggleGroupItem>
			<ToggleGroupItem value="cover" class="flex-1">Cover</ToggleGroupItem>
		</ToggleGroup>
	</div>

	<div class="grid gap-1.5">
		<Label for="resize-mode">Resize algorithm</Label>
		<Select bind:value={resize} type="single">
			<SelectTrigger id="resize-mode">{resizeLabel}</SelectTrigger>
			<SelectContent>
				{#each RESIZE_MODES as r (r.id)}
					<SelectItem value={r.id}>{r.label}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
	</div>

	<div class="grid gap-1.5">
		<Label for="alpha-mode">Alpha mode</Label>
		<Select bind:value={alpha} type="single">
			<SelectTrigger id="alpha-mode">{alphaLabel}</SelectTrigger>
			<SelectContent>
				{#each ALPHA_MODES as a (a.id)}
					<SelectItem value={a.id}>{a.label}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
	</div>

	<div class="grid gap-1.5">
		<div class="flex items-center justify-between">
			<Label for="alpha-threshold">Alpha threshold</Label>
			<span class="text-xs text-muted-foreground tabular-nums">{alphaThreshold}</span>
		</div>
		<Slider
			type="single"
			bind:value={alphaThreshold}
			min={0}
			max={255}
			step={1}
			disabled={alpha !== 'preserve'}
			aria-label="Alpha threshold"
		/>
	</div>
</section>
