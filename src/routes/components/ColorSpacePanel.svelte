<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { COLOR_SPACES } from './sample-data';
	import { colorSpace } from '$lib/stores/app';

	type Props = { compact?: boolean; hideHeading?: boolean };
	let { compact = false, hideHeading = false }: Props = $props();

	let space = $state(colorSpace.get());
	const current = $derived(COLOR_SPACES.find((s) => s.id === space) ?? COLOR_SPACES[0]);
	const triggerLabel = $derived(current.label);

	$effect(() => {
		colorSpace.set(space);
	});
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
		<Select bind:value={space} type="single">
			<SelectTrigger id="color-space">{triggerLabel}</SelectTrigger>
			<SelectContent>
				{#each COLOR_SPACES as s (s.id)}
					<SelectItem value={s.id}>{s.label}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
	</div>

	<figure
		class="relative aspect-[4/3] w-full overflow-hidden border border-border bg-background"
		aria-label="{current.label} 3D visualizer"
	>
		<div
			class="absolute inset-0 bg-[radial-gradient(circle_at_30%_30%,oklch(0.78_0.18_75),transparent_55%),radial-gradient(circle_at_70%_70%,oklch(0.55_0.22_280),transparent_55%),radial-gradient(circle_at_60%_30%,oklch(0.72_0.20_140),transparent_50%)]"
			aria-hidden="true"
		></div>
		<figcaption class="absolute right-2 bottom-2 bg-background/85 px-2 py-0.5 text-xs">
			{current.label}
		</figcaption>
	</figure>

	<div class="grid gap-1.5">
		<p class="text-sm leading-relaxed">{current.short}</p>
		<p
			class="border-l-2 border-border bg-muted/50 px-3 py-1.5 font-mono text-xs text-muted-foreground"
		>
			{current.math}
		</p>
	</div>
</section>
