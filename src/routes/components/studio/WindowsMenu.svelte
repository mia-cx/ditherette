<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import {
		DropdownMenu,
		DropdownMenuCheckboxItem,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuSeparator,
		DropdownMenuTrigger
	} from '$lib/components/ui/dropdown-menu';
	import { WINDOWS, openWindows, resetLayout, toggleWindow } from './workspace';
	import AppWindowIcon from 'phosphor-svelte/lib/AppWindow';
</script>

<DropdownMenu>
	<DropdownMenuTrigger>
		{#snippet child({ props })}
			<Button {...props} size="sm" variant="ghost">
				<AppWindowIcon weight="bold" />
				Windows
			</Button>
		{/snippet}
	</DropdownMenuTrigger>
	<DropdownMenuContent align="end" class="w-44">
		{#each WINDOWS as window (window.id)}
			<DropdownMenuCheckboxItem
				checked={$openWindows.has(window.id)}
				onCheckedChange={() => toggleWindow(window.id)}>{window.title}</DropdownMenuCheckboxItem
			>
		{/each}
		<DropdownMenuSeparator />
		<DropdownMenuItem onSelect={resetLayout}>Reset layout</DropdownMenuItem>
	</DropdownMenuContent>
</DropdownMenu>
