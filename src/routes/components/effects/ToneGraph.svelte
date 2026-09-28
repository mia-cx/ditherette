<script lang="ts">
	import { tonePath, type ToneMap } from '$lib/effects/tone';
	import ToneGrid from './ToneGrid.svelte';

	/** A read-only plot of how an effect maps input tone (x) to output tone (y). */
	type Props = { label: string; curves: readonly { map: ToneMap; class: string }[] };
	let { label, curves }: Props = $props();

	const SIZE = 256;
</script>

<svg
	viewBox="0 0 {SIZE} {SIZE}"
	class="mx-auto aspect-square w-full max-w-48 border border-border bg-muted/30"
	role="img"
	aria-label={label}
>
	<ToneGrid size={SIZE} />
	{#each curves as curve, index (index)}
		<path
			d={tonePath(curve.map, SIZE)}
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			class={curve.class}
		/>
	{/each}
</svg>
