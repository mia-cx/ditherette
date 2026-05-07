<script lang="ts">
	import { Toaster as Sonner, type ToasterProps as SonnerProps } from 'svelte-sonner';
	import { mode } from 'mode-watcher';
	import SpinnerIcon from 'phosphor-svelte/lib/Spinner';
	import CheckCircleIcon from 'phosphor-svelte/lib/CheckCircle';
	import XCircleIcon from 'phosphor-svelte/lib/XCircle';
	import InfoIcon from 'phosphor-svelte/lib/Info';
	import WarningIcon from 'phosphor-svelte/lib/Warning';

	let { ...restProps }: SonnerProps = $props();
</script>

<Sonner
	theme={mode.current}
	class="toaster group"
	style="--normal-bg: var(--color-popover); --normal-text: var(--color-popover-foreground); --normal-border: var(--color-border);"
	{...restProps}
>
	{#snippet loadingIcon()}
		<SpinnerIcon weight="bold" class="size-4 animate-spin" />
	{/snippet}
	{#snippet successIcon()}
		<CheckCircleIcon weight="bold" class="size-4" />
	{/snippet}
	{#snippet errorIcon()}
		<XCircleIcon weight="bold" class="size-4" />
	{/snippet}
	{#snippet infoIcon()}
		<InfoIcon weight="bold" class="size-4" />
	{/snippet}
	{#snippet warningIcon()}
		<WarningIcon weight="bold" class="size-4" />
	{/snippet}
</Sonner>
