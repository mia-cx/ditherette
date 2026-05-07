<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import DownloadIcon from 'phosphor-svelte/lib/DownloadSimple';

	type Props = {
		variant?: 'card' | 'bar';
		hasImage?: boolean;
	};

	let { variant = 'bar', hasImage = false }: Props = $props();
</script>

<section
	class="flex w-full flex-wrap items-center gap-3 {variant === 'card'
		? 'border-border bg-background border p-3'
		: 'bg-background/95 border-border border-t px-3 py-2 backdrop-blur'}"
	aria-label="Export"
>
	<div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
		<Badge variant="outline" class="font-mono">PNG</Badge>
		<Badge variant="outline">512 × 512</Badge>
		<Badge variant="outline">64 colors</Badge>
		<span class="text-muted-foreground hidden text-xs sm:inline">
			Indexed PNG · PLTE + tRNS
		</span>
	</div>

	<Button size="sm" disabled={!hasImage}>
		<DownloadIcon />
		Download PNG
	</Button>
</section>
