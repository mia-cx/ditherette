<script lang="ts">
	import type { DockviewApi, IDockviewGroupPanel } from 'dockview-core';
	import { Button } from '$lib/components/ui/button';
	import { isFloating, toggleFloating } from './dock';
	import ArrowSquareInIcon from 'phosphor-svelte/lib/ArrowSquareIn';
	import ArrowSquareOutIcon from 'phosphor-svelte/lib/ArrowSquareOut';

	/** The float/dock button in each window's tab bar. */
	let { api, group }: { api: DockviewApi; group: IDockviewGroupPanel } = $props();

	let floating = $state(false);

	$effect(() => {
		floating = isFloating(group);
		const listener = group.api.onDidLocationChange(() => (floating = isFloating(group)));
		return () => listener.dispose();
	});
</script>

<div class="flex h-full items-center px-1">
	<Button
		variant="ghost"
		size="icon-xs"
		aria-label={floating ? 'Dock window' : 'Float window'}
		title={floating ? 'Dock window' : 'Float window'}
		onclick={() => toggleFloating(api, group)}
	>
		{#if floating}
			<ArrowSquareInIcon weight="bold" />
		{:else}
			<ArrowSquareOutIcon weight="bold" />
		{/if}
	</Button>
</div>
