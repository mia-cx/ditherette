<script lang="ts">
	import { TRACE, type ScopeChannel } from '$lib/scopes/colour';
	import { LEVELS } from '$lib/scopes/scopes';

	type Channels = readonly [Float32Array, Float32Array, Float32Array];
	type Props = {
		channels: readonly [ScopeChannel, ScopeChannel, ScopeChannel];
		source?: Channels;
		output?: Channels;
	};
	/** One level plot per channel. With both images, the source draws neutral behind the output. */
	let { channels, source, output }: Props = $props();

	const HEIGHT = 100;
	const QUARTERS = [0.25, 0.5, 0.75];
	const rgb = ([r, g, b]: readonly number[], alpha = 1) => `rgb(${r} ${g} ${b} / ${alpha})`;

	/** A filled step outline of `bins`, scaled so the fullest level reaches the top. */
	function area(bins: Float32Array) {
		let peak = 0;
		for (const weight of bins) peak = Math.max(peak, weight);
		if (!peak) return '';
		let path = `M0 ${HEIGHT}`;
		bins.forEach((weight, level) => {
			path += `V${HEIGHT - (weight / peak) * HEIGHT}H${level + 1}`;
		});
		return `${path}V${HEIGHT}Z`;
	}
</script>

<div class="grid size-full grid-rows-3 gap-px bg-white/10">
	{#each channels as channel, index (channel.name)}
		{@const colour = TRACE[channel.name]}
		<div class="relative bg-black">
			<svg
				viewBox="0 0 {LEVELS} {HEIGHT}"
				preserveAspectRatio="none"
				class="absolute inset-0 size-full"
				role="img"
				aria-label="{channel.label} histogram"
			>
				{#each QUARTERS as quarter (quarter)}
					<line
						x1={quarter * LEVELS}
						x2={quarter * LEVELS}
						y2={HEIGHT}
						class="stroke-white/10"
						vector-effect="non-scaling-stroke"
					/>
				{/each}
				{#if source}
					<path
						d={area(source[index]!)}
						fill={output ? 'rgb(255 255 255 / 0.16)' : rgb(colour, 0.3)}
						stroke={output ? 'none' : rgb(colour, 0.9)}
						vector-effect="non-scaling-stroke"
					/>
				{/if}
				{#if output}
					<path
						d={area(output[index]!)}
						fill={rgb(colour, 0.35)}
						stroke={rgb(colour)}
						vector-effect="non-scaling-stroke"
					/>
				{/if}
			</svg>
			<span class="absolute top-1 left-1.5 text-[11px] font-medium" style:color={rgb(colour)}
				>{channel.label}</span
			>
		</div>
	{/each}
</div>
