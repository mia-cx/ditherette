<script lang="ts">
	import { Slider as SliderPrimitive } from 'bits-ui';
	import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		value = $bindable(),
		orientation = 'horizontal',
		class: className,
		...restProps
	}: WithoutChildrenOrChild<SliderPrimitive.RootProps> = $props();

	// Reading bits-ui's snippet props also builds a positioned tick for every step, each measuring
	// the DOM: thousands per change on a fine-stepped slider. Derive the thumbs from the value.
	const thumbIndices = $derived(Array.isArray(value) ? value.map((_, index) => index) : [0]);
</script>

<!--
Discriminated Unions + Destructing (required for bindable) do not
get along, so we shut typescript up by casting `value` to `never`.
-->
<SliderPrimitive.Root
	bind:ref
	bind:value={value as never}
	data-slot="slider"
	{orientation}
	class={cn(
		'relative flex w-full touch-none items-center select-none data-disabled:opacity-50 data-vertical:h-full data-vertical:min-h-40 data-vertical:w-auto data-vertical:flex-col',
		className
	)}
	{...restProps}
>
	<span
		data-slot="slider-track"
		data-orientation={orientation}
		class={cn(
			'relative grow overflow-hidden rounded-none bg-muted bg-muted data-horizontal:h-1 data-horizontal:w-full data-horizontal:w-full data-vertical:h-full data-vertical:h-full data-vertical:w-1'
		)}
	>
		<SliderPrimitive.Range
			data-slot="slider-range"
			class={cn('absolute bg-primary select-none data-horizontal:h-full data-vertical:w-full')}
		/>
	</span>
	{#each thumbIndices as index (index)}
		<SliderPrimitive.Thumb
			data-slot="slider-thumb"
			{index}
			class="relative block size-3 shrink-0 rounded-none border border-ring bg-white ring-ring/50 transition-[color,box-shadow] select-none after:absolute after:-inset-2 hover:ring-1 focus-visible:ring-1 focus-visible:outline-hidden active:ring-1 disabled:pointer-events-none disabled:opacity-50"
		/>
	{/each}
</SliderPrimitive.Root>
