<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import InlineMath from './InlineMath.svelte';
	import { COLOR_SPACES } from './sample-data';
	import { colorSpace } from '$lib/stores/app';

	type Props = { compact?: boolean; hideHeading?: boolean };
	let { compact = false, hideHeading = false }: Props = $props();

	const current = $derived(COLOR_SPACES.find((s) => s.id === $colorSpace) ?? COLOR_SPACES[0]);
	const triggerLabel = $derived(current.label);
</script>

<section class="flex flex-col gap-{compact ? '3' : '4'}" aria-label="Color space controls">
	{#if !hideHeading}
		<div class="flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold tracking-tight">Color space</h2>
			<p class="text-xs text-muted-foreground">How nearest-color is computed.</p>
		</div>
	{/if}

	<div class="grid gap-1.5">
		<Label for="color-space">Distance mode</Label>
		<Select bind:value={$colorSpace} type="single">
			<SelectTrigger
				id="color-space"
				size="auto"
				class={!compact
					? '!w-full max-w-full items-center gap-3 p-3 text-left whitespace-normal'
					: '!w-full'}
			>
				{#if !compact}
					<span class="flex min-w-0 flex-1 items-start gap-3 overflow-hidden text-left">
						<span class="grid min-w-0 flex-1 content-start gap-1 overflow-hidden">
							<span class="truncate text-sm font-medium text-foreground">{current.label}</span>
							<span class="text-xs leading-relaxed whitespace-normal text-muted-foreground"
								>{current.short}</span
							>
							<InlineMath expression={current.latex} />
						</span>
					</span>
				{:else}
					{triggerLabel}
				{/if}
			</SelectTrigger>
			<SelectContent
				class="max-h-[min(32rem,var(--bits-select-content-available-height))] w-(--bits-select-anchor-width) max-w-(--bits-select-anchor-width) overflow-hidden p-0"
			>
				<div class="py-1">
					{#each COLOR_SPACES as s (s.id)}
						<SelectItem value={s.id} label={s.label} class="min-w-0 items-start py-3 pr-8 pl-3">
							<span class="flex min-w-0 flex-1 items-start gap-3 overflow-hidden">
								<span class="grid min-w-0 flex-1 content-start gap-1 overflow-hidden">
									<span class="truncate text-sm font-medium text-foreground">{s.label}</span>
									<span class="text-xs leading-relaxed whitespace-normal text-muted-foreground"
										>{s.short}</span
									>
									<span class="rounded-sm border border-border bg-muted/40 px-2 py-1">
										<InlineMath expression={s.latex} />
									</span>
								</span>
							</span>
						</SelectItem>
					{/each}
				</div>
			</SelectContent>
		</Select>
	</div>
</section>
