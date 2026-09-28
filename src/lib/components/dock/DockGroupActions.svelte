<script lang="ts">
	import type { DockviewApi, IDockviewGroupPanel } from 'dockview-core';
	import { Button } from '$lib/components/ui/button';
	import {
		canCollapse,
		collapsedGroups,
		isFloating,
		sidebarSide,
		toggleCollapsed,
		toggleFloating,
		toggleSidebar
	} from './dock';
	import ArrowSquareInIcon from 'phosphor-svelte/lib/ArrowSquareIn';
	import ArrowSquareOutIcon from 'phosphor-svelte/lib/ArrowSquareOut';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDown';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRight';
	import SidebarSimpleIcon from 'phosphor-svelte/lib/SidebarSimple';

	type Props = {
		api: DockviewApi;
		group: IDockviewGroupPanel;
		/** The window whose column is the main area, which never folds into a strip. */
		main?: string;
	};
	/** The sidebar, collapse, and float/dock buttons in each window's tab bar. */
	let { api, group, main }: Props = $props();

	let floating = $state(false);
	let collapsible = $state(false);
	let side = $state<'left' | 'right'>();
	const collapse = $derived($collapsedGroups[group.id]);
	const folded = $derived(collapse?.axis === 'width');

	$effect(() => {
		const measure = () => {
			floating = isFloating(group);
			collapsible = canCollapse(api, group);
			side = sidebarSide(api, group, main);
		};
		measure();
		const listeners = [group.api.onDidLocationChange(measure), api.onDidLayoutChange(measure)];
		return () => listeners.forEach((listener) => listener.dispose());
	});
</script>

<div class="flex h-full items-center gap-0.5 px-1 [.dv-groupview-header-vertical_&]:flex-col">
	{#if side}
		<Button
			variant="ghost"
			size="icon-xs"
			aria-label={folded ? 'Expand sidebar' : 'Collapse sidebar'}
			aria-expanded={!folded}
			title={folded ? 'Expand sidebar' : 'Collapse sidebar'}
			onclick={() => toggleSidebar(api, group, main)}
		>
			<SidebarSimpleIcon weight="bold" class={side === 'right' ? '-scale-x-100' : ''} />
		</Button>
	{/if}
	{#if collapsible && !folded}
		<Button
			variant="ghost"
			size="icon-xs"
			aria-label={collapse ? 'Expand window' : 'Collapse window'}
			aria-expanded={!collapse}
			title={collapse ? 'Expand window' : 'Collapse window'}
			onclick={() => toggleCollapsed(api, group, main)}
		>
			{#if collapse}
				<CaretRightIcon weight="bold" />
			{:else}
				<CaretDownIcon weight="bold" />
			{/if}
		</Button>
	{/if}
	<Button
		variant="ghost"
		size="icon-xs"
		aria-label={floating ? 'Dock window' : 'Float window'}
		title={floating ? 'Dock window' : 'Float window'}
		onclick={() => toggleFloating(api, group, main)}
	>
		{#if floating}
			<ArrowSquareInIcon weight="bold" />
		{:else}
			<ArrowSquareOutIcon weight="bold" />
		{/if}
	</Button>
</div>
