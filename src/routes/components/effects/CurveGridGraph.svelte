<script lang="ts">
	import type { TwoInputCurve } from 'ditherette';
	import { Button } from '$lib/components/ui/button';
	import { Label } from '$lib/components/ui/label';
	import { channelLabel } from '$lib/effects/catalog';
	import { gridBackdrop } from '$lib/effects/grid-backdrop';
	import {
		canRemoveColumn,
		insertColumn,
		removeColumn,
		setGridValue,
		wraps
	} from '$lib/effects/grid';
	import { hueAxis } from '$lib/effects/tone';

	type Props = {
		id: string;
		curve: TwoInputCurve;
		onchange: (curve: TwoInputCurve) => void;
		/** The point to select, by row and column; the editor sets it after a pick. */
		selected?: { row: number; column: number };
	};
	let { id, curve, onchange, selected = $bindable({ row: 0, column: 0 }) }: Props = $props();

	/** Values sit on the byte grid, like one-input curves. */
	const BYTE = 255;
	const LARGE_STEP = 16;
	const WIDTH = 300;
	const HEIGHT = 200;
	const HIT_RADIUS_PX = 12;
	/** Pixels of vertical drag across the whole value range. */
	const DRAG_PIXELS = 150;
	/** Backdrop resolution: one lattice-coloured cell per 5 × 5 view units. */
	const BACKDROP = { width: 60, height: 40 } as const;

	let svg = $state<SVGSVGElement>();
	let canvas = $state<HTMLCanvasElement>();
	let dragging = $state<{ startY: number; value: number }>();

	const grid = $derived(curve.grid);
	const row = $derived(Math.min(selected.row, grid.rows.length - 1));
	const column = $derived(Math.min(selected.column, grid.columns.length - 1));
	const value = $derived(grid.values[row]![column]!);
	const toByte = (v: number) => Math.round(v * BYTE);
	const onGrid = (v: number) => Math.min(BYTE, Math.max(0, Math.round(v))) / BYTE;
	const at = (position: number, axis: 'x' | 'y') =>
		axis === 'x' ? position * WIDTH : (1 - position) * HEIGHT;
	/** "Hue" rather than "OKLCH · Hue": the chip already names the model. */
	const channelName = (channel: TwoInputCurve['x']) => channelLabel(channel).split(' · ')[1];
	const position = (channel: TwoInputCurve['x'], v: number) =>
		wraps(channel) ? `${Math.round(v * 360)}°` : `${toByte(v)}`;

	// Redraw the backdrop whenever the curve changes; a slow run never overwrites a newer one.
	let drawn = 0;
	$effect(() => {
		const run = ++drawn;
		const target = canvas;
		void gridBackdrop(curve, BACKDROP.width, BACKDROP.height).then(
			(image) => {
				if (run !== drawn || !target) return;
				target.getContext('2d')?.putImageData(image, 0, 0);
			},
			() => {}
		);
	});

	function set(next: number) {
		onchange(setGridValue(curve, row, column, onGrid(next * BYTE)));
	}

	function point(event: PointerEvent) {
		const box = svg!.getBoundingClientRect();
		return {
			x: Math.min(1, Math.max(0, (event.clientX - box.left) / box.width)),
			y: Math.min(1, Math.max(0, 1 - (event.clientY - box.top) / box.height)),
			scale: box.width / WIDTH
		};
	}

	function press(event: PointerEvent) {
		if (event.button !== 0) return;
		const { x, y, scale } = point(event);
		let best: { distance: number; row: number; column: number } | undefined;
		grid.rows.forEach((r, rowIndex) =>
			grid.columns.forEach((c, columnIndex) => {
				const distance = Math.hypot((c - x) * WIDTH, (r - y) * HEIGHT) * scale;
				if (distance < HIT_RADIUS_PX && (!best || distance < best.distance))
					best = { distance, row: rowIndex, column: columnIndex };
			})
		);
		if (!best) return;
		selected = { row: best.row, column: best.column };
		dragging = { startY: event.clientY, value: grid.values[best.row]![best.column]! };
		svg!.setPointerCapture(event.pointerId);
		svg!.querySelector<SVGElement>(`[data-cell="${best.row}-${best.column}"]`)?.focus();
	}

	function drag(event: PointerEvent) {
		if (!dragging) return;
		set(dragging.value + (dragging.startY - event.clientY) / DRAG_PIXELS);
	}

	/** Double-clicking between columns adds one there, sampled from the current surface. */
	function addColumn(event: MouseEvent) {
		const { x } = point(event as PointerEvent);
		const a = wraps(curve.x) ? Math.min(x, 0.999) : x;
		if (grid.columns.some((c) => Math.abs(c - a) * BYTE < 2)) return;
		const inserted = insertColumn(curve, onGrid(a * BYTE));
		if (!inserted) return;
		onchange(inserted.curve);
		selected = { row, column: inserted.index };
	}

	function keydown(event: KeyboardEvent, rowIndex: number, columnIndex: number) {
		const distance = event.shiftKey ? LARGE_STEP : 1;
		const count = grid.columns.length;
		const step = (by: number) =>
			wraps(curve.x)
				? (columnIndex + by + count) % count
				: Math.min(count - 1, Math.max(0, columnIndex + by));
		if (event.key === 'ArrowUp')
			set((toByte(grid.values[rowIndex]![columnIndex]!) + distance) / BYTE);
		else if (event.key === 'ArrowDown')
			set((toByte(grid.values[rowIndex]![columnIndex]!) - distance) / BYTE);
		else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
			selected = { row: rowIndex, column: step(event.key === 'ArrowLeft' ? -1 : 1) };
			svg?.querySelector<SVGElement>(`[data-cell="${rowIndex}-${selected.column}"]`)?.focus();
		} else if (event.key === 'Delete' || event.key === 'Backspace')
			onchange(removeColumn(curve, columnIndex));
		else return;
		event.preventDefault();
	}
</script>

<div class="relative aspect-[3/2] w-full border border-border bg-muted/30">
	<canvas
		bind:this={canvas}
		width={BACKDROP.width}
		height={BACKDROP.height}
		class="absolute inset-0 size-full"
		aria-hidden="true"
	></canvas>
	<svg
		bind:this={svg}
		viewBox="0 0 {WIDTH} {HEIGHT}"
		preserveAspectRatio="none"
		class="absolute inset-0 size-full touch-none select-none"
		role="group"
		aria-label="Curve grid: {channelLabel(curve.x)} across, {channelLabel(curve.x2)} up"
		onpointerdown={press}
		onpointermove={drag}
		onpointerup={() => (dragging = undefined)}
		onpointercancel={() => (dragging = undefined)}
		ondblclick={addColumn}
	>
		{#each grid.columns as c (c)}
			<line
				x1={at(c, 'x')}
				x2={at(c, 'x')}
				y1="0"
				y2={HEIGHT}
				class="stroke-foreground/25"
				vector-effect="non-scaling-stroke"
			/>
		{/each}
		{#each grid.rows as r (r)}
			<line
				x1="0"
				x2={WIDTH}
				y1={at(r, 'y')}
				y2={at(r, 'y')}
				class="stroke-foreground/25"
				vector-effect="non-scaling-stroke"
			/>
		{/each}
		{#each grid.values as cells, rowIndex (rowIndex)}
			{#each cells as cell, columnIndex (columnIndex)}
				{@const isSelected = rowIndex === row && columnIndex === column}
				<rect
					data-cell="{rowIndex}-{columnIndex}"
					x={at(grid.columns[columnIndex]!, 'x') - 4}
					y={at(grid.rows[rowIndex]!, 'y') - 4}
					width="8"
					height="8"
					tabindex="0"
					role="button"
					aria-label="{channelLabel(curve.x)} {position(
						curve.x,
						grid.columns[columnIndex]!
					)}, {channelLabel(curve.x2)} {position(
						curve.x2,
						grid.rows[rowIndex]!
					)}: adjustment {toByte(cell)}"
					aria-pressed={isSelected}
					class="cursor-ns-resize stroke-foreground outline-none focus-visible:stroke-primary {isSelected
						? 'fill-primary'
						: cell === 0.5
							? 'fill-background'
							: 'fill-foreground'}"
					stroke-width="1.5"
					onfocus={() => (selected = { row: rowIndex, column: columnIndex })}
					onkeydown={(event) => keydown(event, rowIndex, columnIndex)}
				/>
			{/each}
		{/each}
	</svg>
</div>
{#if curve.x.channel === 'hue'}
	<div
		class="-mt-2 h-2 border-x border-b border-border"
		style:background-image={hueAxis(curve.x.model)}
	></div>
{/if}

<div class="grid grid-cols-[1fr_1fr_1fr_auto] items-end gap-2">
	<div class="grid gap-1">
		<span class="truncate text-xs text-muted-foreground">{channelName(curve.x)}</span>
		<output
			class="h-8 border border-input bg-muted/30 px-2 text-right font-mono text-xs leading-8 tabular-nums"
			>{position(curve.x, grid.columns[column]!)}</output
		>
	</div>
	<div class="grid gap-1">
		<span class="truncate text-xs text-muted-foreground">{channelName(curve.x2)}</span>
		<output
			class="h-8 border border-input bg-muted/30 px-2 text-right font-mono text-xs leading-8 tabular-nums"
			>{position(curve.x2, grid.rows[row]!)}</output
		>
	</div>
	<div class="grid gap-1">
		<Label for="{id}-grid-value" class="text-xs text-muted-foreground">Adjustment</Label>
		<input
			id="{id}-grid-value"
			class="h-8 w-full border border-input bg-background px-2 text-right font-mono text-xs tabular-nums"
			type="number"
			min="0"
			max={BYTE}
			step="1"
			value={toByte(value)}
			onchange={(event) => set(Number(event.currentTarget.value) / BYTE)}
		/>
	</div>
	<Button
		variant="outline"
		size="sm"
		disabled={!canRemoveColumn(curve, column)}
		onclick={() => onchange(removeColumn(curve, column))}>Remove column</Button
	>
</div>
