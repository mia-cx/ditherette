<script lang="ts">
	import { Select as SelectPrimitive } from 'bits-ui';
	import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';
	import CaretUpIcon from 'phosphor-svelte/lib/CaretUp';

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: WithoutChildrenOrChild<SelectPrimitive.ScrollUpButtonProps> = $props();
</script>

<SelectPrimitive.ScrollUpButton
	bind:ref
	data-slot="select-scroll-up-button"
	class={cn(
		"absolute top-0 z-10 flex h-6 w-full cursor-default items-center justify-center bg-popover/95 [&_svg:not([class*='size-'])]:size-4",
		className
	)}
	{...restProps}
>
	<CaretUpIcon weight="bold" />
</SelectPrimitive.ScrollUpButton>
