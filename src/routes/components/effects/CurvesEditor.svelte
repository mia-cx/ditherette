<script lang="ts">
	import type { CurvesEffect } from 'ditherette';
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import { MAX_CURVE_POINTS, evaluateCurve, type CurvePoint } from '$lib/effects/spline';
	import ChannelToggle from './ChannelToggle.svelte';

	type Props = { id: string; step: CurvesEffect; onchange: (step: CurvesEffect) => void };
	let { id, step, onchange }: Props = $props();

	/** Points sit on the byte grid, so neighbours stay well above the package's 0.001 x gap. */
	const BYTE = 255;
	const LARGE_STEP = 16;
	const SIZE = 256;
	const SAMPLES = 128;
	const HIT_RADIUS_PX = 10;
	const STROKE = {
		rgb: 'text-foreground',
		red: 'text-red-500',
		green: 'text-green-500',
		blue: 'text-blue-500'
	} as const;

	let svg = $state<SVGSVGElement>();
	let selected = $state(0);
	let dragging = $state<number>();

	const points = $derived(step.points);
	const current = $derived(points[Math.min(selected, points.length - 1)]!);
	const path = $derived(
		Array.from({ length: SAMPLES + 1 }, (_, index) => {
			const x = index / SAMPLES;
			return `${index ? 'L' : 'M'}${x * SIZE} ${(1 - evaluateCurve(points, x)) * SIZE}`;
		}).join('')
	);

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
		onchange({ ...step, points: next });
	}

	function remove(index: number) {
		if (points.length <= 2) return;
		onchange({ ...step, points: points.filter((_, other) => other !== index) });
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
			onchange({ ...step, points: next });
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

<div class="grid gap-3">
	<ChannelToggle value={step.channel} onchange={(channel) => onchange({ ...step, channel })} />

	<svg
		bind:this={svg}
		viewBox="0 0 {SIZE} {SIZE}"
		class="aspect-square w-full touch-none border border-border bg-muted/30 select-none"
		role="group"
		aria-label="Curve. Click to add a point; drag points to reshape."
		onpointerdown={press}
		onpointermove={drag}
		onpointerup={() => (dragging = undefined)}
		onpointercancel={() => (dragging = undefined)}
	>
		{#each [0.25, 0.5, 0.75] as line (line)}
			<line x1={line * SIZE} x2={line * SIZE} y1="0" y2={SIZE} class="stroke-border" />
			<line y1={line * SIZE} y2={line * SIZE} x1="0" x2={SIZE} class="stroke-border" />
		{/each}
		<line
			x1="0"
			y1={SIZE}
			x2={SIZE}
			y2="0"
			class="stroke-muted-foreground/40"
			stroke-dasharray="4 4"
		/>
		<path
			d={path}
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			class={STROKE[step.channel]}
		/>
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
