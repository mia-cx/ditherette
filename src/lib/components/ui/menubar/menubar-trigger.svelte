<script lang="ts">
	import { Menubar as MenubarPrimitive } from 'bits-ui';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: MenubarPrimitive.TriggerProps = $props();
</script>

<MenubarPrimitive.Trigger
	bind:ref
	data-slot="menubar-trigger"
	class={cn(
		'flex items-center rounded-none px-1.5 py-[calc(--spacing(0.8))] text-xs font-medium outline-hidden select-none hover:bg-muted aria-expanded:bg-muted',
		className
	)}
	{...restProps}
/>
