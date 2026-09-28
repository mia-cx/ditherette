<script lang="ts">
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { CHANNEL_TONE } from '$lib/effects/catalog';
	import { hueAxis } from '$lib/effects/tone';
	import {
		BINS,
		HISTOGRAM_MODELS,
		outputHistogram,
		sourceHistogram,
		type HistogramChannel,
		type HistogramModel
	} from '$lib/histogram';
	import { processedImage, sourceImageData, uiSettings } from '$lib/stores/app';

	const PLOT_HEIGHT = 100;
	const CMY = { cyan: 'text-cyan-500', magenta: 'text-fuchsia-500', yellow: 'text-yellow-500' };
	const stroke = (name: HistogramChannel['name']) =>
		name === 'cyan' || name === 'magenta' || name === 'yellow'
			? CMY[name]
			: CHANNEL_TONE[name].stroke;

	const settings = $derived({
		model: 'srgb' as HistogramModel,
		source: true,
		output: true,
		...$uiSettings.histogram
	});
	const model = $derived(
		HISTOGRAM_MODELS.find(({ id }) => id === settings.model) ?? HISTOGRAM_MODELS[0]!
	);
	const set = (patch: Partial<typeof settings>) =>
		uiSettings.set({ ...uiSettings.get(), histogram: { ...settings, ...patch } });

	const source = $derived(
		settings.source && $sourceImageData ? sourceHistogram($sourceImageData, model.id) : undefined
	);
	const output = $derived(
		settings.output && $processedImage ? outputHistogram($processedImage, model.id) : undefined
	);

	/** A filled step outline of `bins`, scaled so the fullest bin reaches the top. */
	function area(bins: Float64Array) {
		const peak = Math.max(...bins);
		if (!peak) return '';
		let path = `M0 ${PLOT_HEIGHT}`;
		bins.forEach((weight, bin) => {
			const y = PLOT_HEIGHT - (weight / peak) * PLOT_HEIGHT;
			path += `V${y}H${bin + 1}`;
		});
		return `${path}V${PLOT_HEIGHT}Z`;
	}
</script>

<div class="grid grid-cols-1 gap-3">
	<div class="grid grid-cols-[4rem_minmax(0,1fr)] items-center gap-2">
		<Label for="histogram-model" class="text-xs text-muted-foreground">Model</Label>
		<Select
			type="single"
			value={model.id}
			onValueChange={(id) => set({ model: id as HistogramModel })}
		>
			<SelectTrigger id="histogram-model" class="w-full">{model.label}</SelectTrigger>
			<SelectContent>
				{#each HISTOGRAM_MODELS as option (option.id)}
					<SelectItem value={option.id}>{option.label}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
	</div>
	<div class="flex items-center gap-4" role="group" aria-label="Images to plot">
		<div class="flex items-center gap-2">
			<Checkbox
				id="histogram-source"
				bind:checked={() => settings.source, (source) => set({ source })}
			/>
			<Label for="histogram-source" class="text-xs">
				<span class="size-2.5 rounded-xs bg-foreground/25"></span>Source
			</Label>
		</div>
		<div class="flex items-center gap-2">
			<Checkbox
				id="histogram-output"
				bind:checked={() => settings.output, (output) => set({ output })}
			/>
			<Label for="histogram-output" class="text-xs">
				<span class="size-2.5 rounded-xs border border-foreground bg-foreground/40"></span>Output
			</Label>
		</div>
	</div>

	{#if !$sourceImageData}
		<p class="text-xs text-muted-foreground">Open an image to see its histogram.</p>
	{:else}
		{#each model.channels as channel, index (channel.name)}
			{@const hue = channel.name === 'hue' ? model.id : undefined}
			<figure class="grid gap-1">
				<figcaption class="text-xs text-muted-foreground">{channel.label}</figcaption>
				<svg
					viewBox="0 0 {BINS} {PLOT_HEIGHT}"
					preserveAspectRatio="none"
					class="h-16 w-full rounded-sm bg-muted/40 {stroke(channel.name)}"
					role="img"
					aria-label="{channel.label} histogram"
				>
					{#if source}
						<path d={area(source[index]!)} class="fill-foreground/25" />
					{/if}
					{#if output}
						<path
							d={area(output[index]!)}
							class="fill-current/40 stroke-current"
							stroke-width="1"
							vector-effect="non-scaling-stroke"
						/>
					{/if}
				</svg>
				{#if hue === 'hsl' || hue === 'hsv' || hue === 'oklch' || hue === 'cielch'}
					<div class="h-1.5 rounded-full" style:background={hueAxis(hue)}></div>
				{/if}
			</figure>
		{/each}
	{/if}
</div>
