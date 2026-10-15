<script lang="ts">
	import { SOCIAL_IMAGE, SITE_NAME, canonicalUrl, jsonLdScript } from '$lib/seo';

	type Props = {
		title: string;
		description: string;
		/** Site path the canonical and og:url point at, e.g. `/guides/dithering`. */
		path: string;
		jsonLd?: Record<string, unknown>;
	};
	let { title, description, path, jsonLd }: Props = $props();

	const canonical = $derived(canonicalUrl(path));
	const imageUrl = $derived(canonicalUrl(SOCIAL_IMAGE.path));
</script>

<svelte:head>
	<title>{title}</title>
	<meta name="description" content={description} />
	<link rel="canonical" href={canonical} />
	<meta property="og:site_name" content={SITE_NAME} />
	<meta property="og:type" content="website" />
	<meta property="og:locale" content="en_GB" />
	<meta property="og:title" content={title} />
	<meta property="og:description" content={description} />
	<meta property="og:url" content={canonical} />
	<meta property="og:image" content={imageUrl} />
	<meta property="og:image:width" content={String(SOCIAL_IMAGE.width)} />
	<meta property="og:image:height" content={String(SOCIAL_IMAGE.height)} />
	<meta property="og:image:alt" content={SOCIAL_IMAGE.alt} />
	<meta name="twitter:card" content="summary_large_image" />
	{#if jsonLd}
		<!-- jsonLdScript escapes `<`, so the serialised data cannot break out of the tag. -->
		<!-- eslint-disable-next-line svelte/no-at-html-tags -->
		{@html jsonLdScript(jsonLd)}
	{/if}
</svelte:head>
