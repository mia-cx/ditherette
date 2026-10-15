<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button';
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

<div class="flex min-h-svh flex-col bg-background text-foreground">
	<header class="flex h-10 w-full items-center gap-3 border-b border-border bg-background px-3">
		<a href={resolve('/')} class="text-sm font-semibold tracking-tight">ditherette</a>
		<nav>
			<a
				href={resolve('/guides')}
				aria-current={page.url.pathname === '/guides' ? 'page' : undefined}
				class="text-sm text-muted-foreground transition-colors hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
				>Guides</a
			>
		</nav>
		<Button href="/" size="sm" class="ml-auto">Open editor</Button>
	</header>
	<main class="mx-auto w-full max-w-2xl px-4 py-10">
		{@render children()}
	</main>
</div>
