<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import ThemeToggle from './ThemeToggle.svelte';
	import UploadIcon from 'phosphor-svelte/lib/UploadSimple';
	import ShieldIcon from 'phosphor-svelte/lib/ShieldCheck';
	import GridIcon from 'phosphor-svelte/lib/GridFour';

	type Props = {
		hasImage?: boolean;
		dense?: boolean;
		extras?: import('svelte').Snippet;
	};

	let { hasImage = false, dense = false, extras }: Props = $props();
</script>

<header
	class="bg-background border-border sticky top-0 z-30 flex w-full items-center gap-2 border-b px-3 {dense
		? 'h-11'
		: 'h-12'} sm:px-4"
>
	<a href="/" class="flex items-center gap-2 font-semibold tracking-tight">
		<GridIcon weight="fill" class="text-primary size-5" />
		<span class="text-sm sm:text-base">ditherette</span>
	</a>

	<Badge variant="outline" class="hidden gap-1.5 sm:inline-flex">
		<ShieldIcon weight="fill" class="text-primary" />
		<span class="text-muted-foreground font-normal">Everything runs in your browser</span>
	</Badge>

	<div class="ml-auto flex items-center gap-1.5">
		{#if extras}
			{@render extras()}
		{/if}
		<Button size="sm" variant={hasImage ? 'outline' : 'default'}>
			<UploadIcon />
			{hasImage ? 'Replace Image' : 'Upload Image'}
		</Button>
		<ThemeToggle />
	</div>
</header>
