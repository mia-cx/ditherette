<script lang="ts">
	import { TRACE, type ScopeChannel } from '$lib/scopes/colour';
	import {
		combined,
		LEVELS,
		paint,
		underLayer,
		WAVE_COLUMNS,
		type Layer
	} from '$lib/scopes/scopes';
	import DensityCanvas from './DensityCanvas.svelte';

	type Channels = readonly [Float32Array, Float32Array, Float32Array];
	type Props = {
		channels: readonly [ScopeChannel, ScopeChannel, ScopeChannel];
		under?: Channels;
		over?: Channels;
		/** Each channel in its own column, instead of all three overlaid. */
		parade: boolean;
	};
	/**
	 * Level against image column. With two images, the earlier stage draws neutral behind the
	 * later one.
	 */
	let { channels, under, over, parade }: Props = $props();

	const GRID = [100, 75, 50, 25, 0];
	const rgb = ([r, g, b]: readonly number[]) => `rgb(${r} ${g} ${b})`;

	/** The layers for the given channels: the earlier stage neutral under the later one. */
	function layers(indices: readonly number[]): Layer[] {
		const coloured = (grids: Channels) =>
			indices.map((index) => ({ density: grids[index]!, colour: TRACE[channels[index]!.name] }));
		const below = under
			? over
				? [underLayer(combined(indices.map((index) => under[index]!)))]
				: coloured(under)
			: [];
		return [...below, ...(over ? coloured(over) : [])];
	}

	const plots = $derived(
		(parade ? [[0], [1], [2]] : [[0, 1, 2]]).map((indices) => ({
			indices,
			image: paint(WAVE_COLUMNS, LEVELS, layers(indices))
		}))
	);
</script>

<div class="flex size-full gap-1.5">
	<div
		class="flex flex-col justify-between py-0 text-right text-[10px] leading-none text-white/45 tabular-nums"
		aria-hidden="true"
	>
		{#each GRID as label (label)}<span class="-my-[0.3em]">{label}</span>{/each}
	</div>
	<div
		class="grid flex-1 gap-px bg-white/10"
		style:grid-template-columns="repeat({plots.length}, 1fr)"
	>
		{#each plots as plot (plot.indices.join())}
			<div
				class="relative bg-black"
				role="img"
				aria-label="{plot.indices.map((index) => channels[index]!.label).join(', ')} waveform"
			>
				<DensityCanvas image={plot.image} />
				<svg viewBox="0 0 1 100" preserveAspectRatio="none" class="absolute inset-0 size-full">
					{#each GRID as level (level)}
						<line
							x2="1"
							y1={100 - level}
							y2={100 - level}
							class="stroke-white/12"
							vector-effect="non-scaling-stroke"
						/>
					{/each}
				</svg>
				<div class="absolute top-1 right-1.5 flex gap-2 text-[11px] font-medium">
					{#each plot.indices as index (index)}
						<span style:color={rgb(TRACE[channels[index]!.name])}>{channels[index]!.label}</span>
					{/each}
				</div>
			</div>
		{/each}
	</div>
</div>
