<script lang="ts">
	import { planePoint, type VectorPlane } from '$lib/scopes/colour';
	import { paint, PLANE_SIZE, underLayer, VECTOR_EXTENT, type Scatter } from '$lib/scopes/scopes';
	import { Toggle } from '$lib/components/ui/toggle';
	import DensityCanvas from './DensityCanvas.svelte';

	type Props = {
		plane: VectorPlane;
		source?: Scatter;
		output?: Scatter;
		/** How many times the plot is magnified, for images with little saturation. */
		zoom: number;
		onzoom: (zoom: number) => void;
	};
	/** A colour plane with its primary and secondary targets. Greys sit at the centre. */
	let { plane, source, output, zoom, onzoom }: Props = $props();
	const ZOOMED = 2;

	type Rgb = readonly [number, number, number];
	const TARGETS: readonly { label: string; rgb: Rgb }[] = [
		{ label: 'R', rgb: [1, 0, 0] },
		{ label: 'Yl', rgb: [1, 1, 0] },
		{ label: 'G', rgb: [0, 1, 0] },
		{ label: 'Cy', rgb: [0, 1, 1] },
		{ label: 'B', rgb: [0, 0, 1] },
		{ label: 'Mg', rgb: [1, 0, 1] }
	];
	const FULL = 255;
	const THREE_QUARTERS = 191;
	/** A mid skin tone. The line through it is where most skin falls, whatever the complexion. */
	const SKIN: Rgb = [228, 155, 128];
	const TARGET_SIZE = 0.07;
	const LABEL_OFFSET = 1.16;
	const EXTENT = VECTOR_EXTENT;

	const image = $derived(
		paint(PLANE_SIZE, PLANE_SIZE, [
			...(source
				? [output ? underLayer(source.density) : { density: source.density, colour: source.sums }]
				: []),
			...(output ? [{ density: output.density, colour: output.sums }] : [])
		])
	);

	/** A colour's spot in the graticule's units: up is +y, the circle's edge at radius 1. */
	function at([r, g, b]: Rgb, level = 1) {
		const [u, v] = planePoint(plane, r * level, g * level, b * level, new Float64Array(3));
		return [u * zoom, -v * zoom] as const;
	}
	const targets = $derived(
		TARGETS.map(({ label, rgb }) => ({
			label,
			full: at(rgb, FULL),
			threeQuarters: at(rgb, THREE_QUARTERS)
		}))
	);
	const skin = $derived.by(() => {
		const [x, y] = at(SKIN);
		const length = Math.hypot(x, y) || 1;
		return [x / length, y / length] as const;
	});
</script>

<div class="[container-type:size] grid size-full place-items-center">
	<div class="relative aspect-square w-[min(100cqw,100cqh)] bg-black">
		<DensityCanvas {image} />
		<svg
			viewBox="{-EXTENT} {-EXTENT} {2 * EXTENT} {2 * EXTENT}"
			class="absolute inset-0 size-full"
			role="img"
			aria-label="Vectorscope, {plane.label}"
		>
			<circle r="1" class="fill-none stroke-white/25" vector-effect="non-scaling-stroke" />
			<circle r="0.5" class="fill-none stroke-white/8" vector-effect="non-scaling-stroke" />
			<line x1="-1" x2="1" class="stroke-white/10" vector-effect="non-scaling-stroke" />
			<line y1="-1" y2="1" class="stroke-white/10" vector-effect="non-scaling-stroke" />
			<line
				x2={skin[0]}
				y2={skin[1]}
				class="stroke-amber-200/45"
				stroke-dasharray="4 3"
				vector-effect="non-scaling-stroke"
			/>
			{#each targets as target (target.label)}
				<rect
					x={target.full[0] - TARGET_SIZE / 2}
					y={target.full[1] - TARGET_SIZE / 2}
					width={TARGET_SIZE}
					height={TARGET_SIZE}
					class="fill-none stroke-white/55"
					vector-effect="non-scaling-stroke"
				/>
				<circle
					cx={target.threeQuarters[0]}
					cy={target.threeQuarters[1]}
					r="0.014"
					class="fill-white/45"
				/>
				<text
					x={target.full[0] * LABEL_OFFSET}
					y={target.full[1] * LABEL_OFFSET}
					text-anchor="middle"
					dominant-baseline="central"
					font-size="0.07"
					class="fill-white/60 font-medium">{target.label}</text
				>
			{/each}
		</svg>
		<span class="absolute bottom-1.5 left-2 text-[10px] text-white/45">{plane.label}</span>
		<Toggle
			size="sm"
			bind:pressed={() => zoom === ZOOMED, (pressed) => onzoom(pressed ? ZOOMED : 1)}
			aria-label="Magnify 2×"
			class="absolute top-1.5 right-1.5 h-6 min-w-0 px-2 text-[11px] text-white/55 hover:bg-white/10 hover:text-white aria-pressed:bg-white/15 aria-pressed:text-white data-[state=on]:bg-white/15"
			>2×</Toggle
		>
	</div>
</div>
