<svelte:options namespace="svg" />

<script lang="ts">
	import type { Neutral } from '$lib/effects/tone';

	/**
	 * Quarter gridlines and the dashed neutral line behind a curve, inside an SVG: the identity
	 * diagonal for tone curves, a flat line at the midpoint for adjustment curves, or along the top
	 * for mask curves at full strength.
	 */
	let { size, neutral = 'diagonal' }: { size: number; neutral?: Neutral } = $props();

	const start = $derived({ diagonal: size, flat: size / 2, full: 0 }[neutral]);
	const end = $derived({ diagonal: 0, flat: size / 2, full: 0 }[neutral]);
</script>

{#each [0.25, 0.5, 0.75] as line (line)}
	<line x1={line * size} x2={line * size} y1="0" y2={size} class="stroke-border" />
	<line y1={line * size} y2={line * size} x1="0" x2={size} class="stroke-border" />
{/each}
<line
	x1="0"
	y1={start}
	x2={size}
	y2={end}
	class="stroke-muted-foreground/40"
	stroke-dasharray="4 4"
/>
