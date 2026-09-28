<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Label } from '$lib/components/ui/label';
	import {
		CURVE_CHANNELS,
		samePoints,
		type ChannelCurves,
		type CurveChannel,
		type CurvePoints
	} from '$lib/effects/catalog';
	import { MAX_CURVE_POINTS, evaluateCurve, type CurvePoint } from '$lib/effects/spline';
	import { tonePath } from '$lib/effects/tone';
	import ToneGrid from './ToneGrid.svelte';

	type Props = { id: string; step: ChannelCurves; onchange: (step: ChannelCurves) => void };
	let { id, step, onchange }: Props = $props();

	/** Points sit on the byte grid, so neighbours stay well above the package's 0.001 x gap. */
	const BYTE = 255;
	const LARGE_STEP = 16;
	const SIZE = 256;
	const SAMPLES = 128;
	const HIT_RADIUS_PX = 10;
	const CHANNEL = {
		red: {
			label: 'Red',
			stroke: 'text-red-500',
			check: 'data-checked:border-red-500 data-checked:bg-red-500'
		},
		green: {
			label: 'Green',
			stroke: 'text-green-500',
			check: 'data-checked:border-green-500 data-checked:bg-green-500'
		},
		blue: {
			label: 'Blue',
			stroke: 'text-blue-500',
			check: 'data-checked:border-blue-500 data-checked:bg-blue-500'
		}
	} as const satisfies Record<CurveChannel, { label: string; stroke: string; check: string }>;

	let svg = $state<SVGSVGElement>();
	let selected = $state(0);
	let dragging = $state<number>();
	/** Checked channels. Edits start from the first one's curve and write to all of them. */
	let editing = $state<CurveChannel[]>([...CURVE_CHANNELS]);

	const points = $derived(step.curves[editing[0]!]);
	const current = $derived(points[Math.min(selected, points.length - 1)]!);
	const stroke = $derived(editing.length === 1 ? CHANNEL[editing[0]!].stroke : 'text-foreground');
	/** Channels whose curve differs from the edited one, drawn thin behind it. */
	const others = $derived(
		CURVE_CHANNELS.filter((channel) => !samePoints(step.curves[channel], points))
	);
	const curvePath = (curve: CurvePoints) => tonePath((x) => evaluateCurve(curve, x), SIZE, SAMPLES);

	function setEditing(channel: CurveChannel, checked: boolean) {
		editing = CURVE_CHANNELS.filter((other) =>
			other === channel ? checked : editing.includes(other)
		);
	}

	function write(next: CurvePoints) {
		const curves: Record<CurveChannel, CurvePoints> = { ...step.curves };
		for (const channel of editing) curves[channel] = next;
		onchange({ ...step, curves });
	}

	const toByte = (value: number) => Math.round(value * BYTE);
	const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

	/** Move one point on the byte grid, keeping it strictly between its neighbours. */
	function place(index: number, xByte: number, yByte: number) {
		if (!Number.isFinite(xByte) || !Number.isFinite(yByte)) return;
		const min = index > 0 ? toByte(points[index - 1]![0]) + 1 : 0;
		const max = index < points.length - 1 ? toByte(points[index + 1]![0]) - 1 : BYTE;
		const next = points.slice();
		next[index] = [
			clamp(Math.round(xByte), min, max) / BYTE,
			clamp(Math.round(yByte), 0, BYTE) / BYTE
		];
		write(next);
	}

	function remove(index: number) {
		if (points.length <= 2) return;
		write(points.filter((_, other) => other !== index));
		selected = Math.max(0, index - 1);
	}

	function pointerBytes(event: PointerEvent) {
		const box = svg!.getBoundingClientRect();
		return {
			x: clamp((event.clientX - box.left) / box.width, 0, 1) * BYTE,
			y: clamp(1 - (event.clientY - box.top) / box.height, 0, 1) * BYTE,
			pixelsPerByte: box.width / BYTE
		};
	}

	/** Grab the nearest point, or add one on the curve's grid and drag it. */
	function press(event: PointerEvent) {
		if (event.button !== 0) return;
		const { x, y, pixelsPerByte } = pointerBytes(event);
		const distances = points.map(([px, py]) => Math.hypot(px * BYTE - x, py * BYTE - y));
		const nearest = distances.indexOf(Math.min(...distances));
		let index = nearest;
		if (distances[nearest]! * pixelsPerByte > HIT_RADIUS_PX) {
			const xByte = Math.round(x);
			const taken = points.some(([px]) => Math.abs(toByte(px) - xByte) < 1);
			if (points.length >= MAX_CURVE_POINTS || taken) return;
			index = points.findIndex(([px]) => toByte(px) > xByte);
			if (index < 0) index = points.length;
			const next: CurvePoint[] = points.slice();
			next.splice(index, 0, [xByte / BYTE, Math.round(y) / BYTE]);
			write(next);
		}
		selected = index;
		dragging = index;
		svg!.setPointerCapture(event.pointerId);
		svg!.querySelector<SVGElement>(`[data-point="${index}"]`)?.focus();
	}

	function drag(event: PointerEvent) {
		if (dragging === undefined) return;
		const { x, y } = pointerBytes(event);
		place(dragging, x, y);
	}

	function keydown(event: KeyboardEvent, index: number) {
		const distance = event.shiftKey ? LARGE_STEP : 1;
		const [x, y] = [toByte(points[index]![0]), toByte(points[index]![1])];
		const moves: Record<string, [number, number]> = {
			ArrowLeft: [-distance, 0],
			ArrowRight: [distance, 0],
			ArrowUp: [0, distance],
			ArrowDown: [0, -distance]
		};
		if (event.key in moves) {
			const [dx, dy] = moves[event.key]!;
			place(index, x + dx, y + dy);
		} else if (event.key === 'Delete' || event.key === 'Backspace') remove(index);
		else return;
		event.preventDefault();
	}
</script>

<div class="grid grid-cols-1 gap-3">
	<div class="flex items-center gap-4" role="group" aria-label="Channels to edit">
		{#each CURVE_CHANNELS as channel (channel)}
			<div class="flex items-center gap-2">
				<Checkbox
					id="{id}-{channel}"
					class={CHANNEL[channel].check}
					bind:checked={() => editing.includes(channel), (checked) => setEditing(channel, checked)}
					disabled={editing.length === 1 && editing[0] === channel}
				/>
				<Label for="{id}-{channel}" class="text-xs">{CHANNEL[channel].label}</Label>
			</div>
		{/each}
	</div>

	<svg
		bind:this={svg}
		viewBox="0 0 {SIZE} {SIZE}"
		class="aspect-square w-full touch-none border border-border bg-muted/30 select-none"
		role="group"
		aria-label="Curve. Click to add a point, and drag points to reshape it."
		onpointerdown={press}
		onpointermove={drag}
		onpointerup={() => (dragging = undefined)}
		onpointercancel={() => (dragging = undefined)}
	>
		<ToneGrid size={SIZE} />
		{#each others as channel (channel)}
			<path
				d={curvePath(step.curves[channel])}
				fill="none"
				stroke="currentColor"
				stroke-width="1.5"
				class="{CHANNEL[channel].stroke} opacity-70"
			/>
		{/each}
		<path d={curvePath(points)} fill="none" stroke="currentColor" stroke-width="2" class={stroke} />
		{#each points as [x, y], index (index)}
			<rect
				data-point={index}
				x={x * SIZE - 5}
				y={(1 - y) * SIZE - 5}
				width="10"
				height="10"
				tabindex="0"
				role="button"
				aria-label="Point {index + 1}: input {toByte(x)}, output {toByte(y)}"
				aria-pressed={index === selected}
				class="cursor-grab stroke-foreground outline-none focus-visible:stroke-primary {index ===
				selected
					? 'fill-primary'
					: 'fill-background'}"
				stroke-width="1.5"
				onfocus={() => (selected = index)}
				onkeydown={(event) => keydown(event, index)}
				ondblclick={() => remove(index)}
			/>
		{/each}
	</svg>

	<div class="grid grid-cols-[1fr_1fr_auto] items-end gap-2">
		<div class="grid gap-1">
			<Label for="{id}-point-input" class="text-xs text-muted-foreground">Input</Label>
			<input
				id="{id}-point-input"
				class="h-8 w-full border border-input bg-background px-2 text-right font-mono text-xs tabular-nums"
				type="number"
				min="0"
				max={BYTE}
				step="1"
				value={toByte(current[0])}
				onchange={(event) => place(selected, Number(event.currentTarget.value), toByte(current[1]))}
			/>
		</div>
		<div class="grid gap-1">
			<Label for="{id}-point-output" class="text-xs text-muted-foreground">Output</Label>
			<input
				id="{id}-point-output"
				class="h-8 w-full border border-input bg-background px-2 text-right font-mono text-xs tabular-nums"
				type="number"
				min="0"
				max={BYTE}
				step="1"
				value={toByte(current[1])}
				onchange={(event) => place(selected, toByte(current[0]), Number(event.currentTarget.value))}
			/>
		</div>
		<Button
			variant="outline"
			size="sm"
			disabled={points.length <= 2}
			onclick={() => remove(selected)}>Remove point</Button
		>
	</div>
</div>
