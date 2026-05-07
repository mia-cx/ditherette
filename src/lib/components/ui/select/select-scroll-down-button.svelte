<script lang="ts">
	import { Select as SelectPrimitive } from 'bits-ui';
	import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDown';

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: WithoutChildrenOrChild<SelectPrimitive.ScrollDownButtonProps> = $props();
</script>

<SelectPrimitive.ScrollDownButton
	bind:ref
	data-slot="select-scroll-down-button"
	class={cn(
		"absolute bottom-0 z-10 flex h-6 w-full cursor-default items-center justify-center bg-popover/95 [&_svg:not([class*='size-'])]:size-4",
		className
	)}
	{...restProps}
>
	<CaretDownIcon weight="bold" />
</SelectPrimitive.ScrollDownButton>
