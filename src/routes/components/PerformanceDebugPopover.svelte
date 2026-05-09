<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import { Button, buttonVariants } from '$lib/components/ui/button';
	import { Popover, PopoverContent, PopoverTrigger } from '$lib/components/ui/popover';
	import { Separator } from '$lib/components/ui/separator';
	import {
		clearProcessingMetrics,
		currentProcessingMetrics,
		processingError,
		processingMetricsHistory,
		processingProgress,
		processingTimingSummary
	} from '$lib/stores/app';

	let open = $state(false);

	const latest = $derived($currentProcessingMetrics);
	const status = $derived(
		$processingError ? 'error' : $processingProgress ? 'processing' : latest ? 'ready' : 'idle'
	);
	const scopeLabel = $derived(
		latest ? `${latest.resize} · ${latest.outputPixels.toLocaleString()} px` : 'No samples'
	);

	function formatMs(value: number | undefined) {
		if (!value || !Number.isFinite(value)) return '0ms';
		if (value < 10) return `${value.toFixed(2)}ms`;
		if (value < 100) return `${value.toFixed(1)}ms`;
		return `${Math.round(value)}ms`;
	}

	function formatBytes(value: number | undefined) {
		const bytes = value ?? 0;
		if (bytes < 1024) return `${bytes} B`;
		const units = ['KiB', 'MiB', 'GiB'];
		let current = bytes / 1024;
		for (const unit of units) {
			if (current < 1024) return `${current.toFixed(current < 10 ? 1 : 0)} ${unit}`;
			current /= 1024;
		}
		return `${current.toFixed(1)} TiB`;
	}

	function statusClass() {
		if (status === 'error') return 'bg-destructive';
		if (status === 'processing') return 'bg-blue-500';
		if (status === 'ready') return 'bg-emerald-500';
		return 'bg-muted-foreground';
	}
</script>

<div role="presentation" onpointerenter={() => (open = true)} onpointerleave={() => (open = false)}>
	<Popover bind:open>
		<PopoverTrigger
			class={buttonVariants({ variant: 'ghost', size: 'sm' })}
			aria-label="Show performance metrics"
			onfocus={() => (open = true)}
		>
			<span class="size-2 rounded-full {statusClass()}"></span>
			Perf
		</PopoverTrigger>
		<PopoverContent align="end" class="w-[min(34rem,calc(100vw-1rem))] p-0 text-xs">
			<div class="grid gap-3 p-3">
				<div class="flex items-start justify-between gap-3">
					<div>
						<h2 class="text-sm font-semibold">Processing metrics</h2>
						<p class="text-muted-foreground">Rolling stats reset per source + output branch.</p>
					</div>
					<Button size="sm" variant="outline" onclick={clearProcessingMetrics}>Reset</Button>
				</div>

				<div class="flex flex-wrap gap-1.5">
					<Badge variant="outline">{status}</Badge>
					<Badge variant="secondary">{$processingMetricsHistory.length} samples</Badge>
					<Badge variant="outline">{scopeLabel}</Badge>
				</div>

				{#if $processingProgress}
					<p class="rounded border border-border bg-muted/50 p-2 font-mono">
						{$processingProgress.stage} · {Math.round($processingProgress.progress * 100)}%
					</p>
				{/if}

				{#if latest}
					<section class="grid gap-2">
						<h3 class="font-medium">Latest</h3>
						<div class="grid grid-cols-2 gap-2 sm:grid-cols-4">
							{@render Metric('Total', formatMs(latest.totalMs))}
							{@render Metric(
								'Worker',
								formatMs(latest.timings.find((item) => item.name === 'main worker round trip')?.ms)
							)}
							{@render Metric(
								'Resize',
								formatMs(latest.timings.find((item) => item.name === 'resize compute')?.ms)
							)}
							{@render Metric(
								'Quantize',
								formatMs(latest.timings.find((item) => item.name === 'quantize compute')?.ms)
							)}
						</div>
					</section>

					<Separator />

					<section class="grid gap-2">
						<h3 class="font-medium">Timing history</h3>
						<div class="overflow-x-auto">
							<table class="w-full min-w-[28rem] text-left font-mono text-[0.68rem]">
								<thead class="text-muted-foreground">
									<tr>
										<th class="py-1 pr-2 font-medium">stage</th>
										<th class="py-1 pr-2 text-right font-medium">avg</th>
										<th class="py-1 pr-2 text-right font-medium">p95</th>
										<th class="py-1 pr-2 text-right font-medium">p98</th>
										<th class="py-1 text-right font-medium">p99</th>
									</tr>
								</thead>
								<tbody>
									{#each $processingTimingSummary.slice(0, 8) as summary (summary.name)}
										<tr class="border-t border-border/60">
											<td class="py-1 pr-2">{summary.name}</td>
											<td class="py-1 pr-2 text-right">{formatMs(summary.average)}</td>
											<td class="py-1 pr-2 text-right">{formatMs(summary.p95)}</td>
											<td class="py-1 pr-2 text-right">{formatMs(summary.p98)}</td>
											<td class="py-1 text-right">{formatMs(summary.p99)}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					</section>

					<Separator />

					<section class="grid gap-2 sm:grid-cols-2">
						<div class="grid gap-1">
							<h3 class="font-medium">Cache delta</h3>
							{@render Stat(
								'Resize',
								`${latest.cache.delta.resizedHits} hit / ${latest.cache.delta.resizedMisses} miss`
							)}
							{@render Stat(
								'Derived',
								`${latest.cache.delta.derivedHits} hit / ${latest.cache.delta.derivedMisses} miss`
							)}
							{@render Stat(
								'Palette vectors',
								`${latest.cache.delta.paletteVectorHits} hit / ${latest.cache.delta.paletteVectorMisses} miss`
							)}
							{@render Stat(
								'Evicted',
								`${latest.cache.delta.derivedEvictions} derived / ${latest.cache.delta.resizedEvictions} resize`
							)}
						</div>
						<div class="grid gap-1">
							<h3 class="font-medium">Cache lifetime</h3>
							{@render Stat('Branches', `${latest.cache.lifetime.branchCount}`)}
							{@render Stat(
								'Branch bytes',
								`${formatBytes(latest.cache.lifetime.branchBytes)} / ${formatBytes(latest.cache.lifetime.branchMaxBytes)}`
							)}
							{@render Stat('Resize hits', `${latest.cache.lifetime.resizedHits}`)}
							{@render Stat('Derived hits', `${latest.cache.lifetime.derivedHits}`)}
						</div>
					</section>

					<Separator />

					<section class="grid gap-1">
						<h3 class="font-medium">Memory shape</h3>
						<div class="grid gap-1 sm:grid-cols-2">
							{@render Stat('Source RGBA', formatBytes(latest.memory.sourceBytes))}
							{@render Stat('Resized RGBA', formatBytes(latest.memory.resizedBytes))}
							{@render Stat('Indices', formatBytes(latest.memory.indexBytes))}
							{@render Stat('Color vectors', formatBytes(latest.memory.vectorBytes))}
							{@render Stat('Dither work', formatBytes(latest.memory.ditherWorkBytes))}
							{@render Stat('Branch cache', formatBytes(latest.memory.branchCacheBytes))}
						</div>
					</section>
				{:else}
					<p class="rounded border border-dashed border-border p-3 text-muted-foreground">
						Process an image to collect timing and cache metrics.
					</p>
				{/if}
			</div>
		</PopoverContent>
	</Popover>
</div>

{#snippet Metric(label: string, value: string)}
	<div class="rounded border border-border bg-muted/40 p-2">
		<div class="text-muted-foreground">{label}</div>
		<div class="font-mono text-sm">{value}</div>
	</div>
{/snippet}

{#snippet Stat(label: string, value: string)}
	<div class="flex items-center justify-between gap-3 rounded bg-muted/40 px-2 py-1 font-mono">
		<span class="text-muted-foreground">{label}</span>
		<span class="text-right">{value}</span>
	</div>
{/snippet}
