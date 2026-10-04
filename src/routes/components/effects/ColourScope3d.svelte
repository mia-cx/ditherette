<script lang="ts">
	import { onMount } from 'svelte';
	import { Toggle } from '$lib/components/ui/toggle';
	import { OKLAB_SPREAD, colourCloud } from '$lib/scopes/colour-cloud';
	import { orbitView, type OrbitView } from '$lib/scopes/orbit-view';

	type Props = {
		source: ImageData | undefined;
		/** The live effects: each pixel's colour index and each colour's result. */
		effects: { pixels: Uint32Array; results: Uint32Array } | undefined;
	};
	let { source, effects }: Props = $props();

	/** Hue spokes on the floor, at OKLCH hues, in roughly their own colours. */
	const SPOKES = [
		[29, [0.94, 0.45, 0.42]],
		[60, [0.87, 0.55, 0.26]],
		[105, [0.72, 0.64, 0.2]],
		[142, [0.45, 0.72, 0.35]],
		[195, [0.2, 0.72, 0.72]],
		[264, [0.4, 0.6, 0.95]],
		[305, [0.68, 0.5, 0.9]],
		[340, [0.88, 0.45, 0.72]]
	] as const;
	const FLOOR = -0.5;
	/** Every nth sample draws a before ghost or a motion trail, so they stay readable. */
	const GHOST_EVERY = 4;
	const TRAIL_EVERY = 60;
	/** Trails shorter than this, in world units, are noise. */
	const TRAIL_MIN = 0.01;
	/** The scope turns on its own after this long without input. */
	const IDLE_MS = 2500;
	const SPIN = 0.0025;

	let canvas = $state<HTMLCanvasElement>();
	let showBefore = $state(false);
	let showMotion = $state(true);
	let view: OrbitView | undefined;
	let lastInput = 0;
	let dragging: { x: number; y: number } | undefined;

	const cloud = $derived(source ? colourCloud(source, effects) : undefined);

	function guides() {
		const lines: number[] = [0, FLOOR, 0, 0, -FLOOR, 0];
		const colours: number[] = [1, 1, 1, 0.35, 1, 1, 1, 0.35];
		for (const radius of [0.1, 0.2])
			for (let k = 0; k < 64; k++) {
				const [a0, a1] = [(k / 64) * 2 * Math.PI, ((k + 1) / 64) * 2 * Math.PI];
				const r = radius * OKLAB_SPREAD;
				lines.push(
					Math.cos(a0) * r,
					FLOOR,
					Math.sin(a0) * r,
					Math.cos(a1) * r,
					FLOOR,
					Math.sin(a1) * r
				);
				colours.push(1, 1, 1, 0.12, 1, 1, 1, 0.12);
			}
		for (const [degrees, [r, g, b]] of SPOKES) {
			const angle = (degrees / 180) * Math.PI;
			const reach = 0.2 * OKLAB_SPREAD;
			lines.push(0, FLOOR, 0, Math.cos(angle) * reach, FLOOR, Math.sin(angle) * reach);
			colours.push(r, g, b, 0.7, r, g, b, 0.7);
		}
		return { lines, colours };
	}

	function draw() {
		if (!view) return;
		view.begin();
		const { lines, colours } = guides();
		if (cloud) {
			const { before, after, colours: tint, count } = cloud;
			if (showBefore) {
				const ghost: number[] = [];
				for (let k = 0; k < count; k += GHOST_EVERY)
					ghost.push(before[k * 3]!, before[k * 3 + 1]!, before[k * 3 + 2]!);
				view.draw(
					'points',
					new Float32Array(ghost),
					new Float32Array((ghost.length / 3) * 4).fill(0.14),
					{
						size: 2,
						depth: false
					}
				);
			}
			if (showMotion)
				for (let k = 0; k < count; k += TRAIL_EVERY) {
					const [from, to] = [before.subarray(k * 3, k * 3 + 3), after.subarray(k * 3, k * 3 + 3)];
					if (Math.hypot(to[0]! - from[0]!, to[1]! - from[1]!, to[2]! - from[2]!) < TRAIL_MIN)
						continue;
					lines.push(...from, ...to);
					colours.push(1, 1, 1, 0.18, 1, 1, 1, 0.18);
				}
			view.draw('points', after, tint, { size: 1.6 });
		}
		view.draw('lines', new Float32Array(lines), new Float32Array(colours), { depth: false });
	}

	onMount(() => {
		view = orbitView(canvas!, { yaw: 0.6, pitch: 0.32, distance: 1.75, target: [0, -0.05, 0] });
		let frame = 0;
		const tick = () => {
			if (view && !dragging && performance.now() - lastInput > IDLE_MS) {
				view.turn(SPIN);
				draw();
			}
			frame = requestAnimationFrame(tick);
		};
		frame = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(frame);
	});

	$effect(() => {
		void [cloud, showBefore, showMotion];
		draw();
	});
</script>

<div class="relative size-full min-h-0">
	<canvas
		bind:this={canvas}
		class="absolute inset-0 size-full touch-none"
		aria-label="3D colour scope in Oklab: every sampled pixel after the effects"
		onpointerdown={(event) => {
			lastInput = performance.now();
			dragging = { x: event.clientX, y: event.clientY };
			canvas!.setPointerCapture(event.pointerId);
		}}
		onpointermove={(event) => {
			if (!dragging || !view) return;
			lastInput = performance.now();
			view.orbit(event.clientX - dragging.x, event.clientY - dragging.y);
			dragging = { x: event.clientX, y: event.clientY };
			draw();
		}}
		onpointerup={() => (dragging = undefined)}
		onwheel={(event) => {
			event.preventDefault();
			lastInput = performance.now();
			view?.zoom(event.deltaY);
			draw();
		}}
	></canvas>
	<div class="absolute top-2 right-2 flex gap-1">
		<Toggle size="sm" variant="outline" bind:pressed={showBefore}>Before</Toggle>
		<Toggle size="sm" variant="outline" bind:pressed={showMotion}>Motion</Toggle>
	</div>
</div>
