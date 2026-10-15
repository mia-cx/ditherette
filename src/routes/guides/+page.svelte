<script lang="ts">
	import { resolve } from '$app/paths';
	import { Button } from '$lib/components/ui/button';
	import Seo from '$lib/components/Seo.svelte';
	import { GUIDES, guidePath } from '$lib/guides';
</script>

<Seo
	title="Guides to Wplace templates, palettes and dithering | Ditherette"
	description="Guides to preparing pixel art with Ditherette: Wplace templates and colours, dithering methods, and reducing an image to a fixed palette."
	path="/guides"
/>

<h1 class="text-3xl font-semibold tracking-tight">Guides</h1>
<p class="mt-3 text-muted-foreground">
	Ditherette turns images into palette-limited pixel art in your browser. Each guide covers one part
	of that job and what to choose.
</p>

<ul class="mt-8 list-none space-y-6 p-0">
	{#each GUIDES as guide (guide.slug)}
		<li>
			<h2 class="text-lg font-medium">
				<a
					href={resolve(guidePath(guide.slug))}
					class="underline-offset-4 hover:underline focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
					>{guide.title}</a
				>
			</h2>
			<p class="mt-1 text-sm text-muted-foreground">{guide.description}</p>
		</li>
	{/each}
</ul>

<div class="mt-10">
	<Button href="/" size="lg">Open the editor</Button>
</div>
