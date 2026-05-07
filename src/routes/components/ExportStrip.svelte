<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { downloadIndexedPng } from '$lib/processing/png';
	import { processedImage, processingProgress, sourceMeta } from '$lib/stores/app';
	import DownloadIcon from 'phosphor-svelte/lib/DownloadSimple';

	type Props = { variant?: 'card' | 'bar'; hasImage?: boolean };
	let { variant = 'bar', hasImage = false }: Props = $props();

	const canExport = $derived(Boolean(hasImage && $processedImage && !$processingProgress));
	const sizeLabel = $derived(
		$processedImage ? `${$processedImage.width} × ${$processedImage.height}` : 'No output'
	);
	const colorLabel = $derived($processedImage ? `${$processedImage.palette.length} colors` : '—');

	function download() {
		if (!$processedImage) return;
		downloadIndexedPng($processedImage, exportFilename());
	}

	function exportFilename() {
		const original = $sourceMeta?.name.replace(/\.[^.]+$/, '') || 'image';
		const safeName =
			original
				.trim()
				.replace(/[^a-z0-9._-]+/gi, '-')
				.replace(/^-+|-+$/g, '') || 'image';
		return `${safeName}-ditherette-${$processedImage!.width}x${$processedImage!.height}.png`;
	}
</script>

<section
	class="flex w-full flex-wrap items-center gap-3 {variant === 'card'
		? 'border border-border bg-background p-3'
		: 'border-t border-border bg-background/95 px-3 py-2 backdrop-blur'}"
	aria-label="Export"
>
	<div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
		<Badge variant="outline" class="font-mono">PNG</Badge>
		<Badge variant="outline">{sizeLabel}</Badge>
		<Badge variant="outline">{colorLabel}</Badge>
		<span class="hidden text-xs text-muted-foreground sm:inline">Indexed PNG · PLTE + tRNS</span>
	</div>

	<Button size="sm" disabled={!canExport} onclick={download}>
		<DownloadIcon />
		Download PNG
	</Button>
</section>
