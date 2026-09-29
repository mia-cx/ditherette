<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import type { CurvePoints } from 'ditherette';
	import { tick } from 'svelte';
	import {
		MAX_CURVE_POINTS,
		evaluateCurve,
		evaluatePeriodicCurve,
		type CurvePoint
	} from '$lib/effects/spline';
	import { tonePath } from '$lib/effects/tone';
	import ToneGrid from './ToneGrid.svelte';

	type Props = {
		id: string;
		points: CurvePoints;
		onchange: (points: CurvePoints) => void;
		/** Colour class for the edited curve. */
		stroke: string;
		/** Other curves drawn thin behind the edited one. */
		behind?: readonly { points: CurvePoints; stroke: string }[];
		/** Names for the point fields: what x reads and what y sets. */
		axes?: { x: string; y: string };
		/** The dashed line where the curve changes nothing. */
		neutral?: 'diagonal' | 'flat';
		/**
		 * A hue x axis: the curve wraps, its end points stay at 0 and 1 with one shared y, and other
		 * points dragged past one edge continue at the other.
		 */
		periodic?: boolean;
		/** A CSS gradient drawn under the x axis. */
		spectrum?: string;
	};
	let {
		id,
		points,
		onchange,
		stroke,
		behind = [],
		axes = { x: 'Input', y: 'Output' },
		neutral = 'diagonal',
		periodic = false,
		spectrum
	}: Props = $props();

	/** Points sit on the byte grid, so neighbours stay well above the package's 0.001 x gap. */
	const BYTE = 255;
	const LARGE_STEP = 16;
	const SIZE = 256;
	const SAMPLES = 128;
	const HIT_RADIUS_PX = 10;

	let svg = $state<SVGSVGElement>();
	let selected = $state(0);
	let dragging = $state<number>();

	/** `selected` can outlive a shorter curve after a model or channel switch, so fields use this. */
	const active = $derived(Math.min(selected, points.length - 1));
	const current = $derived(points[active]!);
	const curvePath = (curve: CurvePoints) =>
		tonePath(
			(x) => (periodic ? evaluatePeriodicCurve(curve, x) : evaluateCurve(curve, x)),
			SIZE,
			SAMPLES
		);
	const isEnd = (index: number) => periodic && (index === 0 || index === points.length - 1);

	const toByte = (value: number) => Math.round(value * BYTE);
	const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

	/**
	 * Move one point on the byte grid and return its new index. Points stay strictly between their
	 * neighbours, except on a periodic axis, where they wrap around the seam and may pass them.
	 */
	function place(index: number, xByte: number, yByte: number) {
		if (!Number.isFinite(xByte) || !Number.isFinite(yByte)) return index;
		const y = clamp(Math.round(yByte), 0, BYTE) / BYTE;
		if (isEnd(index)) {
			const next = points.slice();
			next[0] = [0, y];
			next[next.length - 1] = [1, y];
			onchange(next);
			return index;
		}
		if (periodic) {
			const target = Math.round(xByte);
			const cycled = ((target % BYTE) + BYTE) % BYTE;
			// Bytes 0 and 255 are the seam: step over it in the direction of travel.
			const wrapped = cycled === 0 ? (target <= 0 ? BYTE - 1 : 1) : cycled;
			const taken = points.some(([px], other) => other !== index && toByte(px) === wrapped);
			const x = taken ? points[index]![0] : wrapped / BYTE;
			const rest = points.filter((_, other) => other !== index);
			const at = rest.findIndex(([px]) => px > x);
			onchange([...rest.slice(0, at), [x, y], ...rest.slice(at)]);
			return at;
		}
		const min = index > 0 ? toByte(points[index - 1]![0]) + 1 : 0;
		const max = index < points.length - 1 ? toByte(points[index + 1]![0]) - 1 : BYTE;
		const next = points.slice();
		next[index] = [clamp(Math.round(xByte), min, max) / BYTE, y];
		onchange(next);
		return index;
	}

	function remove(index: number) {
		if (points.length <= 2 || isEnd(index)) return;
		onchange(points.filter((_, other) => other !== index));
		selected = Math.max(0, index - 1);
	}

	function pointerBytes(event: PointerEvent) {
		const box = svg!.getBoundingClientRect();
		const x = (event.clientX - box.left) / box.width;
		return {
			// Pointer capture keeps reporting past the edges, which a periodic axis wraps.
			x: (periodic ? x : clamp(x, 0, 1)) * BYTE,
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
			onchange(next);
		}
		selected = index;
		dragging = index;
		svg!.setPointerCapture(event.pointerId);
		svg!.querySelector<SVGElement>(`[data-point="${index}"]`)?.focus();
	}

	function drag(event: PointerEvent) {
		if (dragging === undefined) return;
		const { x, y } = pointerBytes(event);
		dragging = selected = place(dragging, x, y);
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
			selected = place(index, x + dx, y + dy);
			// A wrapped point changes places, so keep focus on it.
			if (selected !== index)
				void tick().then(() =>
					svg?.querySelector<SVGElement>(`[data-point="${selected}"]`)?.focus()
				);
		} else if (event.key === 'Delete' || event.key === 'Backspace') remove(index);
		else return;
		event.preventDefault();
	}
</script>

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
	<ToneGrid size={SIZE} {neutral} />
	{#each behind as curve, index (index)}
		<path
			d={curvePath(curve.points)}
			fill="none"
			stroke="currentColor"
			stroke-width="1.5"
			class="{curve.stroke} opacity-70"
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
			aria-label="Point {index + 1}: {axes.x.toLowerCase()} {toByte(
				x
			)}, {axes.y.toLowerCase()} {toByte(y)}"
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
{#if spectrum}
	<div class="-mt-2 h-2 border-x border-b border-border" style:background-image={spectrum}></div>
{/if}

<div class="grid grid-cols-[1fr_1fr_auto] items-end gap-2">
	<div class="grid gap-1">
		<Label for="{id}-point-input" class="text-xs text-muted-foreground">{axes.x}</Label>
		<input
			id="{id}-point-input"
			class="h-8 w-full border border-input bg-background px-2 text-right font-mono text-xs tabular-nums"
			type="number"
			min="0"
			max={BYTE}
			step="1"
			value={toByte(current[0])}
			disabled={isEnd(active)}
			onchange={(event) =>
				(selected = place(active, Number(event.currentTarget.value), toByte(current[1])))}
		/>
	</div>
	<div class="grid gap-1">
		<Label for="{id}-point-output" class="text-xs text-muted-foreground">{axes.y}</Label>
		<input
			id="{id}-point-output"
			class="h-8 w-full border border-input bg-background px-2 text-right font-mono text-xs tabular-nums"
			type="number"
			min="0"
			max={BYTE}
			step="1"
			value={toByte(current[1])}
			onchange={(event) => place(active, toByte(current[0]), Number(event.currentTarget.value))}
		/>
	</div>
	<Button
		variant="outline"
		size="sm"
		disabled={points.length <= 2 || isEnd(active)}
		onclick={() => remove(active)}>Remove point</Button
	>
</div>
