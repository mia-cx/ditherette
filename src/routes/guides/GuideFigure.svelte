<script lang="ts">
	import { cn } from '$lib/utils';

	type GuideImage = {
		src: string;
		alt: string;
		label: string;
		/** Intrinsic pixel size, used for the width/height attributes. */
		width: number;
		height: number;
		pixelated?: boolean;
	};

	type Props = {
		images: GuideImage[];
		caption?: string;
	};
	let { images, caption }: Props = $props();
</script>

<figure class="not-prose my-8">
	<div class={cn('grid gap-3', images.length === 1 ? 'grid-cols-1' : 'grid-cols-2')}>
		{#each images as image (image.src + image.label)}
			<div>
				<img
					src={image.src}
					alt={image.alt}
					width={image.width}
					height={image.height}
					class="h-auto w-full border border-border"
					style:image-rendering={image.pixelated ? 'pixelated' : undefined}
					loading="lazy"
					decoding="async"
				/>
				<p class="mt-1 text-xs text-muted-foreground">{image.label}</p>
			</div>
		{/each}
	</div>
	{#if caption}
		<figcaption class="mt-2 text-xs text-muted-foreground">{caption}</figcaption>
	{/if}
</figure>
