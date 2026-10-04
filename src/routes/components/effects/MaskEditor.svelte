<script lang="ts">
	import type { ColourChannel, CurvePoints, MaskCurve, TwoInputMaskCurve } from 'ditherette';
	import EyeIcon from 'phosphor-svelte/lib/Eye';
	import PlusIcon from 'phosphor-svelte/lib/Plus';
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import { CURVE_CHANNELS, channelKey, channelLabel, sameChannel } from '$lib/effects/catalog';
	import { neutralGrid } from '$lib/effects/grid';
	import {
		MASK_PRESETS,
		MAX_MASK_CURVES,
		fullMaskCurve,
		maskGridBackdrop,
		presetIndex,
		presetValues,
		setPreset
	} from '$lib/effects/mask';
	import { hueAxis } from '$lib/effects/tone';
	import CurveGraph from './CurveGraph.svelte';
	import CurveGridGraph from './CurveGridGraph.svelte';
	import EffectSlider from './EffectSlider.svelte';

	type Props = {
		id: string;
		mask: readonly MaskCurve[];
		/** Receives the new mask; an empty one means full strength everywhere. */
		onchange: (mask: MaskCurve[]) => void;
		/** Whether the Source preview shows this mask. */
		shown: boolean;
		onshow: (shown: boolean) => void;
	};
	let { id, mask, onchange, shown, onshow }: Props = $props();

	/** Which part of the mask the section shows: one preset family, or every curve. */
	let view = $state('Tones');
	let selected = $state(0);
	let selectedPoint = $state(0);
	let cell = $state({ row: 0, column: 0 });

	const PERCENT = 100;
	const preset = $derived(MASK_PRESETS.find(({ label }) => label === view));
	const active = $derived(Math.min(selected, mask.length - 1));
	const curve = $derived(mask[active]);
	const full = $derived(mask.length >= MAX_MASK_CURVES);

	/** A new curve reads the first of these its mask does not read yet. */
	const NEW_INPUTS: readonly ColourChannel[] = [
		{ model: 'oklch', channel: 'lightness' },
		{ model: 'oklch', channel: 'hue' },
		{ model: 'oklch', channel: 'chroma' },
		{ model: 'oklab', channel: 'a' }
	];

	/** A preset's curve is named after it; others by what they read. */
	function curveName(item: MaskCurve) {
		if (item.x2) return `${channelLabel(item.x)} × ${channelLabel(item.x2).split(' · ')[1]}`;
		return (
			MASK_PRESETS.find((family) => presetIndex([item], family) === 0)?.label ??
			channelLabel(item.x)
		);
	}

	function slide(index: number, percent: number) {
		if (!preset) return;
		const values = presetValues(mask, preset);
		values[index] = percent / PERCENT;
		onchange(setPreset(mask, preset, values));
	}

	function replace(next: MaskCurve) {
		onchange(mask.map((other, at) => (at === active ? next : other)));
	}

	function add() {
		const x = NEW_INPUTS.find((input) => !mask.some((item) => sameChannel(item.x, input)));
		onchange([...mask, fullMaskCurve(x ?? NEW_INPUTS[0]!)]);
		selected = mask.length;
		selectedPoint = 0;
	}

	function remove() {
		onchange(mask.filter((_, index) => index !== active));
		selected = Math.max(0, active - 1);
		selectedPoint = 0;
	}

	const channelFor = (key: string) =>
		CURVE_CHANNELS.find(({ channel }) => channelKey(channel) === key)?.channel;

	/** A curve drawn over one input means nothing over another, so a new input starts at full strength. */
	function setInput(key: string) {
		const x = channelFor(key);
		if (!curve || !x || sameChannel(x, curve.x)) return;
		if (curve.x2 && !sameChannel(x, curve.x2))
			replace({ x, x2: curve.x2, grid: neutralGrid(x, curve.x2, 1) });
		else replace(fullMaskCurve(x));
		selectedPoint = 0;
	}

	/** A second input turns the curve into a grid over both inputs; none turns it back. */
	function setSecondInput(key: string) {
		if (!curve) return;
		const x2 = channelFor(key);
		if (!x2) {
			if (curve.x2) replace(fullMaskCurve(curve.x));
			return;
		}
		if (sameChannel(x2, curve.x) || (curve.x2 && sameChannel(x2, curve.x2))) return;
		replace({ x: curve.x, x2, grid: neutralGrid(curve.x, x2, 1) });
		cell = { row: 0, column: 0 };
	}
</script>

<section class="grid grid-cols-1 gap-3" aria-labelledby="{id}-mask">
	<div class="flex items-center justify-between gap-2">
		<h3 id="{id}-mask" class="text-xs font-medium">Mask</h3>
		<Button
			variant={shown ? 'secondary' : 'ghost'}
			size="xs"
			aria-pressed={shown}
			onclick={() => onshow(!shown)}
		>
			<EyeIcon weight="bold" />
			Show mask
		</Button>
	</div>
	<ToggleGroup
		type="single"
		variant="outline"
		size="sm"
		value={view}
		onValueChange={(next) => {
			if (next) view = next;
		}}
		aria-label="Mask view"
		class="w-full"
	>
		{#each MASK_PRESETS as family (family.label)}
			<ToggleGroupItem value={family.label} class="flex-1 text-xs">{family.label}</ToggleGroupItem>
		{/each}
		<ToggleGroupItem value="Curves" class="flex-1 text-xs">Curves</ToggleGroupItem>
	</ToggleGroup>

	{#if preset}
		{@const values = presetValues(mask, preset)}
		<div class="grid gap-2">
			{#each preset.ranges as range, index (range.label)}
				<EffectSlider
					id="{id}-mask-{range.label.toLowerCase()}"
					label={range.label}
					value={Math.round(values[index]! * PERCENT)}
					min={0}
					max={PERCENT}
					step={1}
					unit="%"
					onchange={(percent) => slide(index, percent)}
				/>
			{/each}
		</div>
	{:else}
		<div class="flex flex-wrap gap-1" role="group" aria-label="Mask curves">
			{#each mask as item, index (index)}
				<Button
					variant={index === active ? 'secondary' : 'outline'}
					size="xs"
					aria-pressed={index === active}
					onclick={() => {
						selected = index;
						selectedPoint = 0;
					}}>{curveName(item)}</Button
				>
			{/each}
			<Button variant="ghost" size="xs" disabled={full} onclick={add}>
				<PlusIcon weight="bold" />
				Curve
			</Button>
		</div>

		{#if curve}
			{#key active}
				{#if curve.x2}
					<CurveGridGraph
						id="{id}-mask"
						{curve}
						valueLabel="Strength"
						neutral={1}
						backdrop={maskGridBackdrop}
						bind:selected={cell}
						onchange={(next: TwoInputMaskCurve) => replace(next)}
					/>
				{:else}
					{@const one = curve}
					<CurveGraph
						id="{id}-mask"
						points={one.points}
						bind:selected={selectedPoint}
						stroke="text-primary"
						neutral="full"
						periodic={one.x.channel === 'hue'}
						spectrum={one.x.channel === 'hue' ? hueAxis(one.x.model) : undefined}
						axes={{ x: channelLabel(one.x), y: 'Strength' }}
						onchange={(points: CurvePoints) => replace({ x: one.x, points })}
					/>
				{/if}
			{/key}

			<div class="grid grid-cols-[4.5rem_minmax(0,1fr)] items-center gap-2">
				<Label for="{id}-mask-x" class="text-xs text-muted-foreground">Reads</Label>
				<Select type="single" value={channelKey(curve.x)} onValueChange={setInput}>
					<SelectTrigger id="{id}-mask-x" class="w-full">{channelLabel(curve.x)}</SelectTrigger>
					<SelectContent class="max-h-80">
						{#each CURVE_CHANNELS as option (channelKey(option.channel))}
							<SelectItem value={channelKey(option.channel)}>{option.label}</SelectItem>
						{/each}
					</SelectContent>
				</Select>
				<Label for="{id}-mask-x2" class="text-xs text-muted-foreground">And</Label>
				<Select
					type="single"
					value={curve.x2 ? channelKey(curve.x2) : ''}
					onValueChange={setSecondInput}
				>
					<SelectTrigger id="{id}-mask-x2" class="w-full"
						>{curve.x2 ? channelLabel(curve.x2) : 'Nothing else'}</SelectTrigger
					>
					<SelectContent class="max-h-80">
						<SelectItem value="">Nothing else</SelectItem>
						{#each CURVE_CHANNELS.filter(({ channel }) => !sameChannel(channel, curve.x)) as option (channelKey(option.channel))}
							<SelectItem value={channelKey(option.channel)}>{option.label}</SelectItem>
						{/each}
					</SelectContent>
				</Select>
			</div>

			<div class="flex gap-1">
				<Button variant="outline" size="sm" onclick={remove}>Remove curve</Button>
			</div>
		{/if}
	{/if}
</section>
