<script lang="ts">
	import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { VARIANTS } from '$lib/components/skeletons/variants';
	import GridIcon from 'phosphor-svelte/lib/GridFour';
	import ArrowRightIcon from 'phosphor-svelte/lib/ArrowRight';
</script>

<svelte:head><title>Ditherette · UI skeleton variants</title></svelte:head>

<div class="bg-background min-h-svh">
	<header class="border-border border-b">
		<div class="mx-auto flex max-w-5xl items-center gap-3 px-4 py-4">
			<GridIcon weight="fill" class="text-primary size-6" />
			<div>
				<h1 class="text-base font-semibold tracking-tight">ditherette · UI skeletons</h1>
				<p class="text-muted-foreground text-xs">
					Six layout explorations using shadcn-svelte primitives. Wiring/processing not yet connected.
				</p>
			</div>
			<Badge variant="outline" class="ml-auto font-mono">SKELETON</Badge>
		</div>
	</header>

	<main class="mx-auto max-w-5xl p-4">
		<ul class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
			{#each VARIANTS as v (v.slug)}
				<li>
					<a href="/{v.slug}" class="block">
						<Card class="hover:bg-muted/40 group h-full transition-colors">
							<CardHeader>
								<div class="flex items-baseline justify-between">
									<CardTitle class="text-base">{v.label}</CardTitle>
									<ArrowRightIcon
										class="text-muted-foreground group-hover:text-foreground size-4 transition-transform group-hover:translate-x-0.5"
									/>
								</div>
								<CardDescription>{v.tagline}</CardDescription>
							</CardHeader>
							<CardContent>
								{@render preview(v.slug)}
							</CardContent>
						</Card>
					</a>
				</li>
			{/each}
		</ul>
	</main>
</div>

{#snippet preview(slug: string)}
	{#if slug === 'v1-stack'}
		<div class="border-border bg-muted/30 grid h-32 grid-rows-[auto_1fr_auto_auto] gap-1 border p-1">
			<div class="bg-background h-3 border"></div>
			<div class="bg-background border"></div>
			<div class="grid grid-cols-2 gap-1">
				<div class="bg-background h-6 border"></div>
				<div class="bg-background h-6 border"></div>
			</div>
			<div class="bg-background h-3 border"></div>
		</div>
	{:else if slug === 'v2-sidebar'}
		<div class="border-border bg-muted/30 grid h-32 grid-cols-[auto_1fr] gap-1 border p-1">
			<div class="bg-background w-10 border"></div>
			<div class="grid grid-rows-[auto_1fr_auto] gap-1">
				<div class="bg-background h-3 border"></div>
				<div class="bg-background border"></div>
				<div class="bg-background h-3 border"></div>
			</div>
		</div>
	{:else if slug === 'v3-tabs'}
		<div class="border-border bg-muted/30 grid h-32 grid-rows-[auto_1fr_auto_1fr] gap-1 border p-1">
			<div class="bg-background h-3 border"></div>
			<div class="bg-background border"></div>
			<div class="bg-background h-3 border"></div>
			<div class="bg-background border"></div>
		</div>
	{:else if slug === 'v4-resizable'}
		<div class="border-border bg-muted/30 grid h-32 grid-rows-[auto_1fr] gap-1 border p-1">
			<div class="bg-background h-3 border"></div>
			<div class="grid grid-cols-[1fr_auto_1fr] gap-1">
				<div class="bg-background border"></div>
				<div class="bg-foreground/30 w-1"></div>
				<div class="bg-background border"></div>
			</div>
		</div>
	{:else if slug === 'v5-drawer'}
		<div class="border-border bg-muted/30 grid h-32 grid-rows-[auto_1fr_auto] gap-1 border p-1">
			<div class="bg-background h-3 border"></div>
			<div class="bg-background border"></div>
			<div class="bg-foreground/80 h-4 border"></div>
		</div>
	{:else if slug === 'v6-accordion'}
		<div class="border-border bg-muted/30 grid h-32 grid-rows-[auto_1fr_auto_auto_auto_auto] gap-1 border p-1">
			<div class="bg-background h-3 border"></div>
			<div class="bg-background border"></div>
			<div class="bg-background h-3 border"></div>
			<div class="bg-background h-3 border"></div>
			<div class="bg-background h-3 border"></div>
			<div class="bg-background h-3 border"></div>
		</div>
	{/if}
{/snippet}
