<script lang="ts">
	/**
	 * Checkbox-styled visibility toggle. Identical visual to the
	 * standard shadcn Checkbox (border, focus ring, primary fill when
	 * checked) — only the indicator icon changes:
	 *   checked   → open eye   (color is visible)
	 *   unchecked → crossed eye (color is hidden)
	 *
	 * Always renders an icon, including the unchecked state, so the
	 * meaning of the control is obvious without inferring "empty box =
	 * unchecked".
	 */
	import { Checkbox as CheckboxPrimitive } from 'bits-ui';
	import { cn, type WithoutChildrenOrChild } from '$lib/utils';
	import EyeIcon from 'phosphor-svelte/lib/Eye';
	import EyeSlashIcon from 'phosphor-svelte/lib/EyeSlash';

	let {
		ref = $bindable(null),
		checked = $bindable(true),
		class: className,
		...restProps
	}: WithoutChildrenOrChild<CheckboxPrimitive.RootProps> = $props();
</script>

<CheckboxPrimitive.Root
	bind:ref
	data-slot="checkbox"
	class={cn(
		'border-input dark:bg-input/30 data-checked:bg-primary data-checked:text-primary-foreground dark:data-checked:bg-primary data-checked:border-primary aria-invalid:aria-checked:border-primary aria-invalid:border-destructive dark:aria-invalid:border-destructive/50 focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 flex size-4 items-center justify-center rounded-none border transition-colors group-has-disabled/field:opacity-50 focus-visible:ring-1 aria-invalid:ring-1 peer relative shrink-0 outline-none after:absolute after:-inset-x-3 after:-inset-y-2 disabled:cursor-not-allowed disabled:opacity-50',
		className
	)}
	bind:checked
	{...restProps}
>
	{#snippet children({ checked })}
		<div
			data-slot="checkbox-indicator"
			class="grid place-content-center text-current transition-none [&>svg]:size-3"
		>
			{#if checked}
				<EyeIcon weight="bold" />
			{:else}
				<EyeSlashIcon weight="bold" class="text-muted-foreground" />
			{/if}
		</div>
	{/snippet}
</CheckboxPrimitive.Root>
