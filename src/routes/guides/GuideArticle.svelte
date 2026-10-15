<script lang="ts">
	import { resolve } from '$app/paths';
	import { Button } from '$lib/components/ui/button';
	import Seo from '$lib/components/Seo.svelte';
	import { GUIDES, guidePath, type Guide } from '$lib/guides';
	import type { Snippet } from 'svelte';

	type Props = {
		slug: Guide['slug'];
		children: Snippet;
	};
	let { slug, children }: Props = $props();

	const guide = $derived(GUIDES.find((g) => g.slug === slug)!);
	const others = $derived(GUIDES.filter((g) => g.slug !== slug));
</script>

<Seo title="{guide.title} | Ditherette" description={guide.description} path={guidePath(slug)} />

<article
	class="prose max-w-none prose-zinc dark:prose-invert prose-headings:font-semibold prose-headings:tracking-tight"
>
	<h1>{guide.title}</h1>
	{@render children()}

	<div class="not-prose mt-10">
		<Button href="/" size="lg">Open the editor</Button>
	</div>

	<h2 class="mt-10">More guides</h2>
	<ul class="not-prose mt-4 list-none space-y-4 p-0">
		{#each others as other (other.slug)}
			<li>
				<a
					href={resolve(guidePath(other.slug))}
					class="font-medium text-foreground underline-offset-4 hover:underline focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
					>{other.title}</a
				>
				<p class="mt-1 text-sm text-muted-foreground">{other.description}</p>
			</li>
		{/each}
	</ul>
</article>
