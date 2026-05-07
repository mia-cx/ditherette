<script lang="ts">
	import { resolve } from '$app/paths';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import ThemeToggle from './ThemeToggle.svelte';
	import UploadIcon from 'phosphor-svelte/lib/UploadSimple';
	import ShieldIcon from 'phosphor-svelte/lib/ShieldCheck';
	import GridIcon from 'phosphor-svelte/lib/GridFour';
	import TrashIcon from 'phosphor-svelte/lib/Trash';

	type Props = {
		hasImage?: boolean;
		dense?: boolean;
		extras?: import('svelte').Snippet;
		onChooseImage?: () => void;
		onClear?: () => void | Promise<void>;
	};

	let { hasImage = false, dense = false, extras, onChooseImage, onClear }: Props = $props();
</script>

<header
	class="sticky top-0 z-30 flex w-full items-center gap-2 border-b border-border bg-background px-3 {dense
		? 'h-11'
		: 'h-12'} sm:px-4"
>
	<a href={resolve('/')} class="flex items-center gap-2 font-semibold tracking-tight">
		<GridIcon weight="bold" class="size-5 text-primary" />
		<span class="text-sm sm:text-base">ditherette</span>
	</a>

	<Badge variant="outline" class="hidden gap-1.5 sm:inline-flex">
		<ShieldIcon weight="bold" class="text-primary" />
		<span class="font-normal text-muted-foreground">Everything runs in your browser</span>
	</Badge>

	<div class="ml-auto flex items-center gap-1.5">
		{#if extras}
			{@render extras()}
		{/if}
		{#if hasImage}
			<Button size="sm" variant="ghost" onclick={onClear}>
				<TrashIcon weight="bold" />
				<span class="hidden sm:inline">Clear</span>
			</Button>
		{/if}
		<Button size="sm" variant={hasImage ? 'outline' : 'default'} onclick={onChooseImage}>
			<UploadIcon weight="bold" />
			{hasImage ? 'Replace Image' : 'Upload Image'}
		</Button>
		<ThemeToggle />
	</div>
</header>
