<script lang="ts">
	import type { Curve } from 'ditherette';
	import { onMount } from 'svelte';
	import { channelLabel } from '$lib/effects/catalog';
	import { evaluateGrid, setGridValue } from '$lib/effects/grid';
	import { gridBackdrop } from '$lib/effects/grid-backdrop';
	import { setPointOutput } from '$lib/effects/pick';
	import { evaluateCurve, evaluatePeriodicCurve } from '$lib/effects/spline';
	import { orbitView, type OrbitView, type Vec3 } from '$lib/scopes/orbit-view';

	type Props = { curve: Curve; onchange: (curve: Curve) => void };
	let { curve, onchange }: Props = $props();

	/** Surface samples across the first input and, for two inputs, along the second. */
	const ACROSS = 110;
	const DEEP = 36;
	/** World height of the full change range. */
	const HEIGHT = 0.5;
	const HIT_RADIUS_PX = 12;
	const DRAG_PIXELS = 220;
	const BACKDROP = { width: 60, height: 40 } as const;
	/** OKLCH hues and their rough colours, to tint one-input hue curves and label hue axes. */
	const HUES = [
		['Red', 29, [0.94, 0.45, 0.42]],
		['Orange', 60, [0.87, 0.55, 0.26]],
		['Yellow', 105, [0.72, 0.64, 0.2]],
		['Green', 142, [0.45, 0.72, 0.35]],
		['Cyan', 195, [0.2, 0.72, 0.72]],
		['Blue', 264, [0.4, 0.6, 0.95]],
		['Purple', 305, [0.68, 0.5, 0.9]],
		['Magenta', 340, [0.88, 0.45, 0.72]]
	] as const;

	let canvas = $state<HTMLCanvasElement>();
	let labels = $state<{ text: string; x: number; y: number }[]>([]);
	let backdrop = $state<ImageData>();
	let view: OrbitView | undefined;
	let nodes: { id: number; row?: number; position: Vec3 }[] = [];
	let dragging:
		| { kind: 'orbit'; x: number; y: number }
		| { kind: 'node'; id: number; row?: number; y: number; value: number }
		| undefined;

	const periodic = $derived(!curve.x2 && curve.kind === 'adjust' && curve.x.channel === 'hue');

	/** The curve's value at `a` across and `b` deep. */
	function value(a: number, b: number) {
		if (curve.x2) return evaluateGrid(curve, a, b);
		return periodic ? evaluatePeriodicCurve(curve.points, a) : evaluateCurve(curve.points, a);
	}
	/** How far the curve moves a colour at `a`, `b`, as a height. */
	function height(a: number, b: number) {
		const v = value(a, b);
		return (curve.kind === 'remap' ? v - a : v - 0.5) * HEIGHT;
	}
	const at = (a: number, b: number, h: number): Vec3 => [a - 0.5, h, 0.5 - b];

	/** Two-input surfaces wear the real result colours; one-input ribbons their input's colours. */
	function colourAt(a: number, b: number): [number, number, number] {
		if (curve.x2 && backdrop) {
			const x = Math.min(BACKDROP.width - 1, Math.floor(a * BACKDROP.width));
			const y = Math.min(BACKDROP.height - 1, Math.floor((1 - b) * BACKDROP.height));
			const offset = (y * BACKDROP.width + x) * 4;
			return [
				backdrop.data[offset]! / 255,
				backdrop.data[offset + 1]! / 255,
				backdrop.data[offset + 2]! / 255
			];
		}
		if (curve.x.channel !== 'hue') return [0.15 + a * 0.75, 0.15 + a * 0.75, 0.15 + a * 0.75];
		const degrees = a * 360;
		const next = HUES.findIndex(([, hue]) => hue > degrees);
		const [upper, lower] = [
			HUES[next < 0 ? 0 : next]!,
			HUES[next <= 0 ? HUES.length - 1 : next - 1]!
		];
		const span = (upper[1] - lower[1] + 360) % 360 || 360;
		const t = ((degrees - lower[1] + 360) % 360) / span;
		return [0, 1, 2].map((c) => lower[2][c]! + (upper[2][c]! - lower[2][c]!) * t) as [
			number,
			number,
			number
		];
	}

	function draw() {
		if (!view) return;
		view.begin();
		const deep = curve.x2 ? DEEP : 2;
		const positions = new Float32Array((ACROSS + 1) * deep * 3);
		const colours = new Float32Array((ACROSS + 1) * deep * 4);
		const step = 0.01;
		for (let j = 0; j < deep; j++) {
			const b = j / (deep - 1);
			for (let i = 0; i <= ACROSS; i++) {
				const a = i / ACROSS;
				const k = j * (ACROSS + 1) + i;
				positions.set(at(a, b, height(a, b)), k * 3);
				// Light from the upper left, by the surface's slope.
				const dx =
					(height(Math.min(1, a + step), b) - height(Math.max(0, a - step), b)) / (2 * step);
				const dz = curve.x2
					? (height(a, Math.min(1, b + step)) - height(a, Math.max(0, b - step))) / (2 * step)
					: 0;
				const shade = 0.72 + (0.28 * (0.5 * dx + 0.8 + 0.3 * dz)) / Math.hypot(dx, 1, dz);
				const [r, g, bl] = colourAt(a, b);
				colours.set([r * shade, g * shade, bl * shade, 1], k * 4);
			}
		}
		const indices: number[] = [];
		for (let j = 0; j < deep - 1; j++)
			for (let i = 0; i < ACROSS; i++) {
				const p = j * (ACROSS + 1) + i;
				const q = p + ACROSS + 1;
				indices.push(p, q, p + 1, p + 1, q, q + 1);
			}
		view.draw('triangles', positions, colours, { indices: new Uint32Array(indices) });

		const lines: number[] = [];
		const lineColours: number[] = [];
		const line = (from: Vec3, to: Vec3, alpha: number) => {
			lines.push(...from, ...to);
			lineColours.push(1, 1, 1, alpha, 1, 1, 1, alpha);
		};
		const floor = -HEIGHT / 2 - 0.02;
		for (let k = 0; k <= 8; k++) {
			line([-0.5 + k / 8, floor, -0.5], [-0.5 + k / 8, floor, 0.5], 0.08);
			line([-0.5, floor, -0.5 + k / 8], [0.5, floor, -0.5 + k / 8], 0.08);
		}
		nodes = [];
		if (curve.x2) {
			const { grid } = curve;
			grid.values.forEach((cells, row) =>
				cells.forEach((cell, column) => {
					const position = at(grid.columns[column]!, grid.rows[row]!, (cell - 0.5) * HEIGHT);
					if (cell !== 0.5) line(position, [position[0], 0, position[2]], 0.55);
					nodes.push({ id: column, row, position });
				})
			);
		} else
			curve.points.forEach(([a, v], id) =>
				nodes.push({ id, position: at(a, 1, (curve.kind === 'remap' ? v - a : v - 0.5) * HEIGHT) })
			);
		view.draw('lines', new Float32Array(lines), new Float32Array(lineColours));
		const nodePositions = new Float32Array(nodes.flatMap(({ position }) => [...position]));
		view.draw('points', nodePositions, new Float32Array(nodes.length * 4).fill(1), {
			size: 7,
			round: true,
			depth: false
		});

		const names =
			curve.x.channel === 'hue'
				? HUES.map(([name, degrees]) => [name, degrees / 360] as const)
				: [];
		const across = channelLabel(curve.x);
		labels = [
			...names.map(([text, a]) => ({ text, ...point(at(a, 0, floor), 0.1) })),
			{ text: `${across} →`, ...point([0, floor, 0.86], 0) },
			...(curve.x2 ? [{ text: `${channelLabel(curve.x2)} →`, ...point([-0.74, floor, 0], 0) }] : [])
		];
	}

	function point(position: Vec3, forward: number) {
		const [x, y] = view!.project([position[0], position[1], position[2] + forward]);
		return { x, y };
	}

	$effect(() => {
		const c = curve;
		if (!c.x2) {
			backdrop = undefined;
			return;
		}
		void gridBackdrop(c, BACKDROP.width, BACKDROP.height).then(
			(image) => {
				if (curve === c) backdrop = image;
			},
			() => {}
		);
	});

	$effect(() => {
		void [curve, backdrop];
		draw();
	});

	onMount(() => {
		view = orbitView(canvas!, { yaw: -0.38, pitch: 0.5, distance: 2.05, target: [0.03, 0, 0] });
		draw();
		const resize = new ResizeObserver(() => draw());
		resize.observe(canvas!);
		return () => resize.disconnect();
	});

	function press(event: PointerEvent) {
		canvas!.setPointerCapture(event.pointerId);
		const hit = nodes
			.map((node) => ({
				node,
				distance: Math.hypot(
					...view!.project(node.position).map((c, i) => c - [event.offsetX, event.offsetY][i]!)
				)
			}))
			.filter(({ distance }) => distance < HIT_RADIUS_PX)
			.sort((l, r) => l.distance - r.distance)[0];
		if (!hit) {
			dragging = { kind: 'orbit', x: event.clientX, y: event.clientY };
			return;
		}
		const { id, row } = hit.node;
		const start = curve.x2 ? curve.grid.values[row!]![id]! : curve.points[id]![1];
		dragging = { kind: 'node', id, row, y: event.clientY, value: start };
	}

	function move(event: PointerEvent) {
		if (!dragging || !view) return;
		if (dragging.kind === 'orbit') {
			view.orbit(event.clientX - dragging.x, event.clientY - dragging.y);
			dragging = { ...dragging, x: event.clientX, y: event.clientY };
			draw();
			return;
		}
		const next =
			Math.round(
				Math.min(1, Math.max(0, dragging.value + (dragging.y - event.clientY) / DRAG_PIXELS)) * 255
			) / 255;
		if (curve.x2) onchange(setGridValue(curve, dragging.row!, dragging.id, next));
		else onchange({ ...curve, points: setPointOutput(curve.points, dragging.id, next, periodic) });
	}
</script>

<div class="relative size-full min-h-0">
	<canvas
		bind:this={canvas}
		class="absolute inset-0 size-full touch-none"
		aria-label="3D surface of the curve: how far it moves each input"
		onpointerdown={press}
		onpointermove={move}
		onpointerup={() => (dragging = undefined)}
		onwheel={(event) => {
			event.preventDefault();
			view?.zoom(event.deltaY);
			draw();
		}}
	></canvas>
	{#each labels as label (label.text)}
		<span
			class="pointer-events-none absolute -translate-x-1/2 text-[11px] text-muted-foreground"
			style:left="{label.x}px"
			style:top="{label.y}px">{label.text}</span
		>
	{/each}
</div>
