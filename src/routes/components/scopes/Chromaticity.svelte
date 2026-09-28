<script lang="ts">
	import { D65, SPECTRAL_LOCUS, SRGB_PRIMARIES, XY_BOUNDS } from '$lib/scopes/colour';
	import {
		chromaticityBackdrop,
		paint,
		PLANE_SIZE,
		underLayer,
		type Scatter
	} from '$lib/scopes/scopes';
	import DensityCanvas from './DensityCanvas.svelte';

	type Props = { source?: Scatter; output?: Scatter };
	/** Samples on the CIE 1931 xy diagram, inside the spectral locus, against the sRGB gamut. */
	let { source, output }: Props = $props();

	const LABELLED_NM = [460, 480, 500, 520, 540, 560, 580, 600, 620];
	const LABEL_OFFSET = 0.035;

	const image = $derived(
		paint(PLANE_SIZE, PLANE_SIZE, [
			chromaticityBackdrop(),
			...(source
				? [output ? underLayer(source.density) : { density: source.density, colour: source.sums }]
				: []),
			...(output ? [{ density: output.density, colour: output.sums }] : [])
		])
	);

	/** xy to the graticule's units, with y up. */
	const point = ([x, y]: readonly [number, number]) => [x, XY_BOUNDS.y - y] as const;
	const points = (list: readonly (readonly [number, number])[]) =>
		list.map((xy) => point(xy).join(',')).join(' ');
	const locus = SPECTRAL_LOCUS.map(([, x, y]) => [x, y] as const);
	/** Wavelength labels, pushed outward from the white point so they sit outside the locus. */
	const labels = LABELLED_NM.map((nm) => {
		const [, x, y] = SPECTRAL_LOCUS.find(([wavelength]) => wavelength === nm)!;
		const [dx, dy] = [x - D65[0], y - D65[1]];
		const length = Math.hypot(dx, dy);
		return { nm, at: point([x + (dx / length) * LABEL_OFFSET, y + (dy / length) * LABEL_OFFSET]) };
	});
</script>

<div class="[container-type:size] grid size-full place-items-center">
	<div
		class="relative aspect-[8/9] w-[min(100cqw,calc(100cqh*8/9))] bg-black"
		role="img"
		aria-label="CIE 1931 chromaticity"
	>
		<DensityCanvas {image} />
		<svg
			viewBox="0 0 {XY_BOUNDS.x} {XY_BOUNDS.y}"
			class="absolute inset-0 size-full overflow-visible"
		>
			{#each [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7] as grid (grid)}
				<line
					x1={grid}
					x2={grid}
					y2={XY_BOUNDS.y}
					class="stroke-white/6"
					vector-effect="non-scaling-stroke"
				/>
				<line
					y1={XY_BOUNDS.y - grid}
					y2={XY_BOUNDS.y - grid}
					x2={XY_BOUNDS.x}
					class="stroke-white/6"
					vector-effect="non-scaling-stroke"
				/>
			{/each}
			<polygon
				points={points(locus)}
				class="fill-none stroke-white/45"
				vector-effect="non-scaling-stroke"
			/>
			<polygon
				points={points(SRGB_PRIMARIES)}
				class="fill-none stroke-white/80"
				vector-effect="non-scaling-stroke"
			/>
			<circle
				cx={point(D65)[0]}
				cy={point(D65)[1]}
				r="0.006"
				class="fill-none stroke-white"
				vector-effect="non-scaling-stroke"
			/>
			{#each labels as label (label.nm)}
				<text
					x={label.at[0]}
					y={label.at[1]}
					text-anchor="middle"
					dominant-baseline="central"
					font-size="0.02"
					class="fill-white/45">{label.nm}</text
				>
			{/each}
		</svg>
		<span class="absolute right-2 bottom-1.5 text-[10px] text-white/45">sRGB · D65</span>
	</div>
</div>
