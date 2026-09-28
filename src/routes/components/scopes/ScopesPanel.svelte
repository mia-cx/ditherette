<script lang="ts">
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import { SCOPE_MODELS, vectorPlane, type ScopeModel } from '$lib/scopes/colour';
	import {
		chromaticityScope,
		histogram,
		sampleOutput,
		sampleSource,
		SCOPES,
		vectorscope,
		waveform,
		type ScopeKind
	} from '$lib/scopes/scopes';
	import { processedImage, sourceImageData, uiSettings } from '$lib/stores/app';
	import Chromaticity from './Chromaticity.svelte';
	import Histogram from './Histogram.svelte';
	import Vectorscope from './Vectorscope.svelte';
	import Waveform from './Waveform.svelte';

	const IMAGES = [
		{ id: 'source', label: 'Source' },
		{ id: 'output', label: 'Output' }
	] as const;

	const settings = $derived({
		scope: 'histogram' as ScopeKind,
		model: 'srgb' as ScopeModel,
		source: true,
		output: true,
		vectorZoom: 1,
		...$uiSettings.scopes
	});
	const set = (patch: Partial<typeof settings>) =>
		uiSettings.set({ ...uiSettings.get(), scopes: { ...settings, ...patch } });

	const scope = $derived(SCOPES.find(({ id }) => id === settings.scope) ?? SCOPES[0]);
	const model = $derived(SCOPE_MODELS.find(({ id }) => id === settings.model) ?? SCOPE_MODELS[0]!);
	const chromaticity = $derived(scope.id === 'chromaticity');

	// Sampling depends only on the image and whether it shows, so switching scope or model reuses it.
	const showSource = $derived(settings.source);
	const showOutput = $derived(settings.output);
	const source = $derived(
		showSource && $sourceImageData ? sampleSource($sourceImageData) : undefined
	);
	const output = $derived(
		showOutput && $processedImage ? sampleOutput($processedImage) : undefined
	);
	const plane = $derived(vectorPlane(model.id));
</script>

<div class="flex h-full min-h-0 flex-col gap-2 p-3">
	<div class="flex flex-wrap items-center gap-2">
		<Select type="single" value={scope.id} onValueChange={(id) => set({ scope: id as ScopeKind })}>
			<SelectTrigger aria-label="Scope" class="w-40">{scope.label}</SelectTrigger>
			<SelectContent>
				{#each SCOPES as option (option.id)}
					<SelectItem value={option.id}>{option.label}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
		<Select
			type="single"
			value={model.id}
			disabled={chromaticity}
			onValueChange={(id) => set({ model: id as ScopeModel })}
		>
			<SelectTrigger aria-label="Colour model" class="w-32"
				>{chromaticity ? 'CIE xy' : model.label}</SelectTrigger
			>
			<SelectContent>
				{#each SCOPE_MODELS as option (option.id)}
					<SelectItem value={option.id}>{option.label}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
		<ToggleGroup
			type="multiple"
			variant="outline"
			size="sm"
			aria-label="Images to show"
			class="ml-auto"
			bind:value={
				() => IMAGES.filter(({ id }) => settings[id]).map(({ id }) => id),
				(shown: string[]) => {
					// Keep one image showing; an empty scope has nothing to compare.
					if (shown.length)
						set({ source: shown.includes('source'), output: shown.includes('output') });
				}
			}
		>
			{#each IMAGES as image (image.id)}
				<ToggleGroupItem value={image.id} class="px-3 text-xs">{image.label}</ToggleGroupItem>
			{/each}
		</ToggleGroup>
	</div>

	<div class="relative min-h-56 flex-1 border border-border bg-black text-white">
		{#if !$sourceImageData}
			<p class="absolute inset-0 grid place-items-center text-xs text-white/50">
				Open an image to see its scopes.
			</p>
		{:else if scope.id === 'histogram'}
			<Histogram
				channels={model.channels}
				source={source && histogram(source, model.id)}
				output={output && histogram(output, model.id)}
			/>
		{:else if scope.id === 'waveform' || scope.id === 'parade'}
			<div class="size-full p-2">
				<Waveform
					channels={model.channels}
					parade={scope.id === 'parade'}
					source={source && waveform(source, model.id)}
					output={output && waveform(output, model.id)}
				/>
			</div>
		{:else if scope.id === 'vectorscope'}
			<div class="size-full p-2">
				<Vectorscope
					{plane}
					source={source && vectorscope(source, plane, settings.vectorZoom)}
					output={output && vectorscope(output, plane, settings.vectorZoom)}
					zoom={settings.vectorZoom}
					onzoom={(vectorZoom) => set({ vectorZoom })}
				/>
			</div>
		{:else}
			<div class="size-full p-2">
				<Chromaticity
					source={source && chromaticityScope(source)}
					output={output && chromaticityScope(output)}
				/>
			</div>
		{/if}
	</div>
</div>
