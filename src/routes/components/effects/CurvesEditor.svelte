<script lang="ts">
	import type { Curve, CurvePoints, CurvesEffect, TwoInputCurve } from 'ditherette';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeft';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRight';
	import ArrowsOutIcon from 'phosphor-svelte/lib/ArrowsOut';
	import EyedropperIcon from 'phosphor-svelte/lib/Eyedropper';
	import PlusIcon from 'phosphor-svelte/lib/Plus';
	import { onDestroy } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import {
		CURVE_CHANNELS,
		FLAT,
		MAX_CURVES,
		STRAIGHT,
		channelKey,
		channelLabel,
		neutralCurve,
		sameChannel
	} from '$lib/effects/catalog';
	import { neutralGrid, setGridValue } from '$lib/effects/grid';
	import { channelValue, pickCell, pickPoint, setPointOutput } from '$lib/effects/pick';
	import { hueAxis } from '$lib/effects/tone';
	import { curvePicker, type CurvePicker } from '$lib/stores/curve-pick';
	import CurveGraph from './CurveGraph.svelte';
	import CurveAnalysis from './CurveAnalysis.svelte';
	import CurveGridGraph from './CurveGridGraph.svelte';

	type Props = {
		id: string;
		step: CurvesEffect;
		onchange: (step: CurvesEffect) => void;
		/** The selected curve, shared with the analysis view. */
		selected?: number;
		/** Inside the analysis view, which has no Expand button of its own. */
		expanded?: boolean;
	};
	let { id, step, onchange, selected = $bindable(0), expanded = false }: Props = $props();
	let analysing = $state(false);

	/** Each curve's colour, by position: its line and its chip swatch. */
	const TONES = [
		{ stroke: 'text-primary', swatch: 'bg-primary' },
		{ stroke: 'text-sky-500', swatch: 'bg-sky-500' },
		{ stroke: 'text-fuchsia-500', swatch: 'bg-fuchsia-500' },
		{ stroke: 'text-emerald-500', swatch: 'bg-emerald-500' },
		{ stroke: 'text-orange-500', swatch: 'bg-orange-500' },
		{ stroke: 'text-violet-500', swatch: 'bg-violet-500' }
	] as const;
	const tone = (index: number) => TONES[index % TONES.length]!;

	/** A new curve targets one hue range's saturation, the classic selective adjustment. */
	const NEW_CURVE = neutralCurve(
		{ model: 'oklch', channel: 'hue' },
		{ model: 'oklch', channel: 'chroma' }
	);

	let selectedPoint = $state(0);
	/** `selected` can outlive a removed curve, so everything reads this. */
	const active = $derived(Math.min(selected, step.curves.length - 1));
	const curve = $derived(step.curves[active]);

	/**
	 * "Lightness (OKLCH)" for a remap, "Hue vs Chroma (OKLCH)" for an adjustment, and
	 * "Hue × Lightness vs Chroma (OKLCH)" for a two-input curve.
	 */
	function curveName(c: Curve) {
		const parts = [c.x, ...(c.x2 ? [c.x2] : []), c.y].map((channel) =>
			channelLabel(channel).split(' · ')
		);
		const models = new Set(parts.map(([model]) => model));
		const name = (index: number) =>
			models.size === 1 ? parts[index]![1] : `${parts[index]![1]} (${parts[index]![0]})`;
		const suffix = models.size === 1 ? ` (${parts[0]![0]})` : '';
		if (c.kind === 'remap') return `${parts[0]![1]} (${parts[0]![0]})`;
		const inputs = c.x2 ? `${name(0)} × ${name(1)}` : name(0);
		return `${inputs} vs ${name(parts.length - 1)}${suffix}`;
	}

	/** A hue input wraps for adjustments; hue remaps read their input straight across. */
	const periodic = (c: Curve) => c.kind === 'adjust' && c.x.channel === 'hue';
	/** The selected point of a two-input curve's grid. */
	let cell = $state({ row: 0, column: 0 });

	function replaceAt(index: number, next: Curve) {
		onchange({ ...step, curves: step.curves.map((curve, at) => (at === index ? next : curve)) });
	}

	function replace(next: Curve) {
		grabbed = undefined;
		replaceAt(active, next);
	}

	function add() {
		grabbed = undefined;
		onchange({ ...step, curves: [...step.curves, NEW_CURVE] });
		selected = step.curves.length;
		selectedPoint = 0;
	}

	function remove() {
		grabbed = undefined;
		onchange({ ...step, curves: step.curves.filter((_, index) => index !== active) });
		selected = Math.max(0, active - 1);
		selectedPoint = 0;
	}

	/** Curves apply in list order, so moving one changes the result. */
	function move(from: number, to: number) {
		if (from === to || to < 0 || to >= step.curves.length) return;
		grabbed = undefined;
		const curves = step.curves.slice();
		const [moved] = curves.splice(from, 1);
		curves.splice(to, 0, moved!);
		onchange({ ...step, curves });
		selected = to;
		selectedPoint = 0;
	}
	let dragFrom = $state<number>();

	/** Pixels of vertical drag that move a picked point across the whole output range. */
	const PUSH_PIXELS = 200;
	/** The picked point: its curve, its index or grid cell, and its value when the pick began. */
	let grabbed:
		| { curve: number; point: number; y: number; cell?: { row: number; column: number } }
		| undefined;
	const picker: CurvePicker = {
		pick(rgb) {
			if (!curve) return;
			if (curve.x2) {
				cell = pickCell(curve, rgb);
				grabbed = { curve: active, point: 0, y: curve.grid.values[cell.row]![cell.column]!, cell };
				return;
			}
			const { points, index } = pickPoint(curve, periodic(curve), channelValue(curve.x, rgb));
			grabbed = { curve: active, point: index, y: points[index]![1] };
			selectedPoint = index;
			replaceAt(active, { ...curve, points });
		},
		push(up) {
			if (!grabbed || active !== grabbed.curve) return;
			const pickedCurve = step.curves[grabbed.curve];
			if (!pickedCurve) return;
			const y = grabbed.y + up / PUSH_PIXELS;
			if (pickedCurve.x2) {
				if (!grabbed.cell) return;
				const value = Math.round(Math.min(1, Math.max(0, y)) * 255) / 255;
				replaceAt(
					grabbed.curve,
					setGridValue(pickedCurve, grabbed.cell.row, grabbed.cell.column, value)
				);
				return;
			}
			const points = setPointOutput(pickedCurve.points, grabbed.point, y, periodic(pickedCurve));
			replaceAt(grabbed.curve, { ...pickedCurve, points });
		}
	};
	const picking = $derived($curvePicker === picker);
	onDestroy(() => {
		grabbed = undefined;
		if (curvePicker.get() === picker) curvePicker.set(undefined);
	});

	const channelFor = (key: string) =>
		CURVE_CHANNELS.find(({ channel }) => channelKey(channel) === key)?.channel;

	/** A curve drawn over one input means nothing over another, so a new input starts neutral. */
	function setInput(key: string) {
		const x = channelFor(key);
		if (!curve || !x || sameChannel(x, curve.x)) return;
		if (curve.x2 && !sameChannel(x, curve.x2))
			replace({ kind: 'adjust', x, x2: curve.x2, y: curve.y, grid: neutralGrid(x, curve.x2) });
		else replace(neutralCurve(x, curve.y));
	}

	/** A second input turns the curve into a grid over both inputs; none turns it back. */
	function setSecondInput(key: string) {
		if (!curve) return;
		const x2 = channelFor(key);
		if (!x2) {
			if (curve.x2) replace(neutralCurve(curve.x, curve.y));
			return;
		}
		if (sameChannel(x2, curve.x) || (curve.x2 && sameChannel(x2, curve.x2))) return;
		replace({ kind: 'adjust', x: curve.x, x2, y: curve.y, grid: neutralGrid(curve.x, x2) });
		cell = { row: 0, column: 0 };
	}

	/** A new output keeps an adjustment's shape; a remap onto another channel becomes a flat adjustment. */
	function setOutput(key: string) {
		const y = channelFor(key);
		if (!curve || !y || sameChannel(y, curve.y)) return;
		if (curve.x2) replace({ ...curve, y });
		else if (curve.kind === 'remap') replace({ kind: 'adjust', x: curve.x, y, points: FLAT });
		else if (sameChannel(curve.x, y)) replace(neutralCurve(curve.x, y));
		else replace({ ...curve, y });
	}

	function setKind(kind: string) {
		if (!curve || curve.x2 || kind === curve.kind) return;
		if (kind === 'remap' && sameChannel(curve.x, curve.y))
			replace({ ...curve, kind, points: STRAIGHT });
		if (kind === 'adjust') replace({ ...curve, kind, points: FLAT });
	}
</script>

<div class="grid grid-cols-1 gap-3">
	<div class="flex flex-wrap gap-1" role="group" aria-label="Curves">
		{#each step.curves as item, index (index)}
			<Button
				variant={index === active ? 'secondary' : 'outline'}
				size="xs"
				aria-pressed={index === active}
				draggable="true"
				ondragstart={() => (dragFrom = index)}
				ondragover={(event: DragEvent) => event.preventDefault()}
				ondrop={() => {
					if (dragFrom !== undefined) move(dragFrom, index);
					dragFrom = undefined;
				}}
				onclick={() => {
					grabbed = undefined;
					selected = index;
					selectedPoint = 0;
				}}
			>
				<span class="size-2 {tone(index).swatch}" aria-hidden="true"></span>
				{curveName(item)}
			</Button>
		{/each}
		<Button variant="ghost" size="xs" disabled={step.curves.length >= MAX_CURVES} onclick={add}>
			<PlusIcon weight="bold" />
			Curve
		</Button>
		<Button
			variant={picking ? 'secondary' : 'ghost'}
			size="icon-xs"
			class="ml-auto"
			aria-label="Pick from preview"
			aria-pressed={picking}
			disabled={!curve}
			onclick={() => curvePicker.set(picking ? undefined : picker)}
		>
			<EyedropperIcon weight="bold" />
		</Button>
		{#if !expanded}
			<Button
				variant="ghost"
				size="icon-xs"
				aria-label="Expand"
				disabled={!curve}
				onclick={() => (analysing = true)}
			>
				<ArrowsOutIcon weight="bold" />
			</Button>
		{/if}
	</div>

	{#if curve}
		{#key active}
			{#if curve.x2}
				<CurveGridGraph
					{id}
					{curve}
					bind:selected={cell}
					onchange={(next: TwoInputCurve) => replace(next)}
				/>
			{:else}
				{@const one = curve}
				<CurveGraph
					{id}
					points={one.points}
					bind:selected={selectedPoint}
					stroke={tone(active).stroke}
					neutral={one.kind === 'remap' ? 'diagonal' : 'flat'}
					periodic={periodic(one)}
					spectrum={one.x.channel === 'hue' ? hueAxis(one.x.model) : undefined}
					behind={step.curves.flatMap((other, index) =>
						index !== active && !other.x2 && other.kind === one.kind && sameChannel(other.x, one.x)
							? [{ points: other.points, stroke: tone(index).stroke }]
							: []
					)}
					axes={{
						x: channelLabel(one.x),
						y: one.kind === 'remap' ? channelLabel(one.y) : 'Adjustment'
					}}
					onchange={(points: CurvePoints) => replace({ ...one, points })}
				/>
			{/if}
		{/key}

		<div class="grid grid-cols-[4.5rem_minmax(0,1fr)] items-center gap-2">
			<Label for="{id}-x" class="text-xs text-muted-foreground">Reads</Label>
			<Select type="single" value={channelKey(curve.x)} onValueChange={setInput}>
				<SelectTrigger id="{id}-x" class="w-full">{channelLabel(curve.x)}</SelectTrigger>
				<SelectContent class="max-h-80">
					{#each CURVE_CHANNELS as option (channelKey(option.channel))}
						<SelectItem value={channelKey(option.channel)}>{option.label}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
			<Label for="{id}-x2" class="text-xs text-muted-foreground">And</Label>
			<Select
				type="single"
				value={curve.x2 ? channelKey(curve.x2) : ''}
				onValueChange={setSecondInput}
			>
				<SelectTrigger id="{id}-x2" class="w-full"
					>{curve.x2 ? channelLabel(curve.x2) : 'Nothing else'}</SelectTrigger
				>
				<SelectContent class="max-h-80">
					<SelectItem value="">Nothing else</SelectItem>
					{#each CURVE_CHANNELS.filter(({ channel }) => !sameChannel(channel, curve.x)) as option (channelKey(option.channel))}
						<SelectItem value={channelKey(option.channel)}>{option.label}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
			<Label for="{id}-y" class="text-xs text-muted-foreground">Changes</Label>
			<Select type="single" value={channelKey(curve.y)} onValueChange={setOutput}>
				<SelectTrigger id="{id}-y" class="w-full">{channelLabel(curve.y)}</SelectTrigger>
				<SelectContent class="max-h-80">
					{#each CURVE_CHANNELS as option (channelKey(option.channel))}
						<SelectItem value={channelKey(option.channel)}>{option.label}</SelectItem>
					{/each}
				</SelectContent>
			</Select>
			<span class="text-xs text-muted-foreground">Kind</span>
			<ToggleGroup
				type="single"
				variant="outline"
				size="sm"
				value={curve.kind}
				onValueChange={setKind}
				aria-label="Kind"
				class="w-full"
			>
				<ToggleGroupItem
					value="remap"
					class="flex-1 text-xs"
					disabled={Boolean(curve.x2) || !sameChannel(curve.x, curve.y)}>Remap</ToggleGroupItem
				>
				<ToggleGroupItem value="adjust" class="flex-1 text-xs">Adjust</ToggleGroupItem>
			</ToggleGroup>
		</div>

		<div class="flex gap-1">
			<Button
				variant="outline"
				size="icon-sm"
				aria-label="Move curve earlier"
				disabled={active === 0}
				onclick={() => move(active, active - 1)}><CaretLeftIcon weight="bold" /></Button
			>
			<Button
				variant="outline"
				size="icon-sm"
				aria-label="Move curve later"
				disabled={active === step.curves.length - 1}
				onclick={() => move(active, active + 1)}><CaretRightIcon weight="bold" /></Button
			>
			<Button variant="outline" size="sm" onclick={remove}>Remove curve</Button>
		</div>
	{/if}
</div>

{#if !expanded}
	<CurveAnalysis id="{id}-analysis" {step} {onchange} bind:open={analysing} bind:selected />
{/if}
