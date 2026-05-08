<script lang="ts">
	import type { PaletteColor } from '$lib/processing/types';

	type Props = { colors: readonly PaletteColor[] };
	let { colors }: Props = $props();

	let width = $state(0);

	const swatchPitch = 20;
	const moreLabelWidth = 88;
	const visibleCount = $derived.by(() => {
		if (width <= 0) return colors.length;
		const fullCapacity = Math.floor(width / swatchPitch);
		if (colors.length <= fullCapacity) return colors.length;
		return Math.max(1, Math.floor((width - moreLabelWidth) / swatchPitch));
	});
	const visibleColors = $derived(colors.slice(0, visibleCount));
	const hiddenCount = $derived(Math.max(0, colors.length - visibleCount));
</script>

<span
	bind:clientWidth={width}
	class="flex max-w-full min-w-0 flex-nowrap items-center gap-1 overflow-hidden"
	aria-hidden="true"
>
	{#each visibleColors as color (color.key)}
		<span class="size-4 shrink-0 border border-border" title={color.name}>
			{#if color.kind === 'transparent'}
				<span
					class="block size-full [background-image:repeating-conic-gradient(rgba(0,0,0,0.3)_0%_25%,transparent_0%_50%)] [background-size:6px_6px]"
				></span>
			{:else}
				<span class="block size-full" style="background-color: {color.key}"></span>
			{/if}
		</span>
	{/each}
	{#if hiddenCount > 0}
		<span class="min-w-fit shrink-0 pl-1 text-xs text-muted-foreground"
			>and {hiddenCount} more...</span
		>
	{/if}
</span>
