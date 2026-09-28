<script lang="ts">
	import type { Curve, CurvePoints, CurvesEffect } from 'ditherette';
	import CaretLeftIcon from 'phosphor-svelte/lib/CaretLeft';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRight';
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
	import { channelValue, pickPoint, setPointOutput } from '$lib/effects/pick';
	import { hueAxis } from '$lib/effects/tone';
	import { curvePicker, type CurvePicker } from '$lib/stores/curve-pick';
	import CurveGraph from './CurveGraph.svelte';

	type Props = { id: string; step: CurvesEffect; onchange: (step: CurvesEffect) => void };
	let { id, step, onchange }: Props = $props();

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

	let selected = $state(0);
	/** `selected` can outlive a removed curve, so everything reads this. */
	const active = $derived(Math.min(selected, step.curves.length - 1));
	const curve = $derived(step.curves[active]);

	/** "Lightness (OKLCH)" for a remap, "Hue vs Chroma (OKLCH)" for an adjustment. */
	function curveName({ kind, x, y }: Curve) {
		const [xModel, xName] = channelLabel(x).split(' · ');
		const [yModel, yName] = channelLabel(y).split(' · ');
		if (kind === 'remap') return `${xName} (${xModel})`;
		return xModel === yModel
			? `${xName} vs ${yName} (${xModel})`
			: `${xName} (${xModel}) vs ${yName} (${yModel})`;
	}

	/** A hue input wraps for adjustments; hue remaps read their input straight across. */
	const periodic = (c: Curve) => c.kind === 'adjust' && c.x.channel === 'hue';

	function replace(next: Curve) {
		onchange({ ...step, curves: step.curves.map((c, index) => (index === active ? next : c)) });
	}

	function add() {
		onchange({ ...step, curves: [...step.curves, NEW_CURVE] });
		selected = step.curves.length;
	}

	function remove() {
		onchange({ ...step, curves: step.curves.filter((_, index) => index !== active) });
		selected = Math.max(0, active - 1);
	}

	/** Curves apply in list order, so moving one changes the result. */
	function move(from: number, to: number) {
		if (from === to || to < 0 || to >= step.curves.length) return;
		const curves = step.curves.slice();
		const [moved] = curves.splice(from, 1);
		curves.splice(to, 0, moved!);
		onchange({ ...step, curves });
		selected = to;
	}
	let dragFrom = $state<number>();

	/** Pixels of vertical drag that move a picked point across the whole output range. */
	const PUSH_PIXELS = 200;
	let grabbed: { index: number; y: number } | undefined;
	const picker: CurvePicker = {
		pick(rgb) {
			if (!curve) return;
			const { points, index } = pickPoint(curve, periodic(curve), channelValue(curve.x, rgb));
			grabbed = { index, y: points[index]![1] };
			replace({ ...curve, points });
		},
		push(up) {
			if (!curve || !grabbed) return;
			const points = setPointOutput(
				curve.points,
				grabbed.index,
				grabbed.y + up / PUSH_PIXELS,
				periodic(curve)
			);
			replace({ ...curve, points });
		}
	};
	const picking = $derived($curvePicker === picker);
	onDestroy(() => {
		if (curvePicker.get() === picker) curvePicker.set(undefined);
	});

	const channelFor = (key: string) =>
		CURVE_CHANNELS.find(({ channel }) => channelKey(channel) === key)?.channel;

	/** A curve drawn over one input means nothing over another, so a new input starts neutral. */
	function setInput(key: string) {
		const x = channelFor(key);
		if (curve && x && !sameChannel(x, curve.x)) replace(neutralCurve(x, curve.y));
	}

	/** A new output keeps an adjustment's shape; a remap onto another channel becomes a flat adjustment. */
	function setOutput(key: string) {
		const y = channelFor(key);
		if (!curve || !y || sameChannel(y, curve.y)) return;
		if (curve.kind === 'remap') replace({ kind: 'adjust', x: curve.x, y, points: FLAT });
		else if (sameChannel(curve.x, y)) replace(neutralCurve(curve.x, y));
		else replace({ ...curve, y });
	}

	function setKind(kind: string) {
		if (!curve || kind === curve.kind) return;
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
				onclick={() => (selected = index)}
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
	</div>

	{#if curve}
		{#key active}
			<CurveGraph
				{id}
				points={curve.points}
				stroke={tone(active).stroke}
				neutral={curve.kind === 'remap' ? 'diagonal' : 'flat'}
				periodic={periodic(curve)}
				spectrum={curve.x.channel === 'hue' ? hueAxis(curve.x.model) : undefined}
				behind={step.curves.flatMap((other, index) =>
					index !== active && other.kind === curve.kind && sameChannel(other.x, curve.x)
						? [{ points: other.points, stroke: tone(index).stroke }]
						: []
				)}
				axes={{
					x: channelLabel(curve.x),
					y: curve.kind === 'remap' ? channelLabel(curve.y) : 'Adjustment'
				}}
				onchange={(points: CurvePoints) => replace({ ...curve, points })}
			/>
		{/key}

		<div class="grid grid-cols-[4.5rem_minmax(0,1fr)] items-center gap-2">
			{#each [{ label: 'Reads', suffix: 'x', value: curve.x, set: setInput }, { label: 'Changes', suffix: 'y', value: curve.y, set: setOutput }] as field (field.suffix)}
				<Label for="{id}-{field.suffix}" class="text-xs text-muted-foreground">{field.label}</Label>
				<Select type="single" value={channelKey(field.value)} onValueChange={field.set}>
					<SelectTrigger id="{id}-{field.suffix}" class="w-full"
						>{channelLabel(field.value)}</SelectTrigger
					>
					<SelectContent class="max-h-80">
						{#each CURVE_CHANNELS as option (channelKey(option.channel))}
							<SelectItem value={channelKey(option.channel)}>{option.label}</SelectItem>
						{/each}
					</SelectContent>
				</Select>
			{/each}
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
					disabled={!sameChannel(curve.x, curve.y)}>Remap</ToggleGroupItem
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
