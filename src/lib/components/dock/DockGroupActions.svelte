<script lang="ts">
	import type { DockviewApi, IDockviewGroupPanel } from 'dockview-core';
	import { Button } from '$lib/components/ui/button';
	import { collapsedGroups, isFloating, toggleCollapsed, toggleFloating } from './dock';
	import ArrowSquareInIcon from 'phosphor-svelte/lib/ArrowSquareIn';
	import ArrowSquareOutIcon from 'phosphor-svelte/lib/ArrowSquareOut';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDown';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRight';

	/** The collapse and float/dock buttons in each window's tab bar. */
	let { api, group }: { api: DockviewApi; group: IDockviewGroupPanel } = $props();

	let floating = $state(false);
	const collapsed = $derived(Boolean($collapsedGroups[group.id]));

	$effect(() => {
		floating = isFloating(group);
		const listener = group.api.onDidLocationChange(() => (floating = isFloating(group)));
		return () => listener.dispose();
	});
</script>

<div class="flex h-full items-center gap-0.5 px-1 [.dv-groupview-header-vertical_&]:flex-col">
	<Button
		variant="ghost"
		size="icon-xs"
		aria-label={collapsed ? 'Expand window' : 'Collapse window'}
		aria-expanded={!collapsed}
		title={collapsed ? 'Expand window' : 'Collapse window'}
		onclick={() => toggleCollapsed(api, group)}
	>
		{#if collapsed}
			<CaretRightIcon weight="bold" />
		{:else}
			<CaretDownIcon weight="bold" />
		{/if}
	</Button>
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
