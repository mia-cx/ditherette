<svelte:options namespace="svg" />

<script lang="ts">
	/**
	 * Quarter gridlines and the dashed neutral line behind a curve, inside an SVG: the identity
	 * diagonal for tone curves, or a flat line at the midpoint for adjustment curves.
	 */
	let { size, neutral = 'diagonal' }: { size: number; neutral?: 'diagonal' | 'flat' } = $props();
</script>

{#each [0.25, 0.5, 0.75] as line (line)}
	<line x1={line * size} x2={line * size} y1="0" y2={size} class="stroke-border" />
	<line y1={line * size} y2={line * size} x1="0" x2={size} class="stroke-border" />
{/each}
<line
	x1="0"
	y1={neutral === 'flat' ? size / 2 : size}
	x2={size}
	y2={neutral === 'flat' ? size / 2 : 0}
	class="stroke-muted-foreground/40"
	stroke-dasharray="4 4"
/>
