<script lang="ts">
	import { page } from '$app/state';
	import { Badge } from '$lib/components/ui/badge';
	import { VARIANTS } from './variants';
</script>

<nav
	class="bg-muted/30 border-border flex gap-1 overflow-x-auto border-b px-2 py-1.5"
	aria-label="Skeleton variant"
>
	<a
		href="/"
		class="text-muted-foreground hover:text-foreground hover:bg-muted shrink-0 px-2 py-1 text-xs font-medium"
	>
		← all variants
	</a>
	<div class="bg-border my-1 w-px shrink-0"></div>
	{#each VARIANTS as v (v.slug)}
		{@const active = page.url.pathname.startsWith(`/${v.slug}`)}
		<a
			href="/{v.slug}"
			class="hover:bg-muted shrink-0 px-2 py-1 text-xs font-medium {active
				? 'bg-background border-border border'
				: 'text-muted-foreground hover:text-foreground'}"
			aria-current={active ? 'page' : undefined}
		>
			{v.label}
		</a>
	{/each}
	<div class="ml-auto hidden shrink-0 items-center gap-1 lg:flex">
		<Badge variant="outline" class="font-mono">SKELETON</Badge>
	</div>
</nav>
