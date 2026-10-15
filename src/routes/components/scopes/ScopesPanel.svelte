<script lang="ts">
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';
	import { SCOPE_MODELS, vectorPlane, type ScopeModel } from '$lib/scopes/colour';
	import {
		chromaticityScope,
		clipping,
		formatShare,
		histogram,
		sampleOutput,
		sampleSource,
		SCOPES,
		vectorscope,
		waveform,
		type Samples,
		type ScopeKind
	} from '$lib/scopes/scopes';
	import { effectsStage, pickStages, sampleEffected, STAGES, type Stage } from '$lib/scopes/stages';
	import { exportable } from '$lib/processing/export';
	import { processedImage, sourceImageData, uiSettings } from '$lib/stores/app';
	import Chromaticity from './Chromaticity.svelte';
	import Histogram from './Histogram.svelte';
	import Vectorscope from './Vectorscope.svelte';
	import Waveform from './Waveform.svelte';

	const settings = $derived({
		scope: 'histogram' as ScopeKind,
		model: 'srgb' as ScopeModel,
		source: true,
		effects: false,
		output: true,
		vectorZoom: 1,
		...$uiSettings.scopes
	});
	const set = (patch: Partial<typeof settings>) =>
		uiSettings.set({ ...uiSettings.get(), scopes: { ...settings, ...patch } });

	const scope = $derived(SCOPES.find(({ id }) => id === settings.scope) ?? SCOPES[0]);
	const model = $derived(SCOPE_MODELS.find(({ id }) => id === settings.model) ?? SCOPE_MODELS[0]!);
	const chromaticity = $derived(scope.id === 'chromaticity');

	/** Stages turned on most recently last: dropping the earliest leaves the newest comparison. */
	let recent: Stage[] | undefined;
	const shown = $derived(STAGES.filter(({ id }) => settings[id]).map(({ id }) => id));
	const [underStage, overStage] = $derived<[Stage?, Stage?]>(
		shown.length > 1 ? [shown[0], shown[1]] : [undefined, shown[0]]
	);

	// Sampling depends only on the image and whether it shows, so switching scope or model reuses it.
	const showSource = $derived(settings.source);
	const showEffects = $derived(settings.effects);
	const showOutput = $derived(settings.output);
	const effectsImage = $derived($effectsStage?.image);
	const sourceSamples = $derived(
		(showSource || (showEffects && effectsImage?.kind === 'source')) && $sourceImageData
			? sampleSource($sourceImageData)
			: undefined
	);
	const effectsSamples = $derived(
		!showEffects || !effectsImage
			? undefined
			: effectsImage.kind === 'source'
				? sourceSamples
				: sampleEffected(effectsImage)
	);
	const outputSamples = $derived(
		showOutput && $processedImage ? sampleOutput($processedImage) : undefined
	);
	const samples = $derived<Record<Stage, Samples | undefined>>({
		source: sourceSamples,
		effects: effectsSamples,
		output: outputSamples
	});
	const current = $derived({
		source: true,
		effects: $effectsStage?.current ?? false,
		output: $exportable
	});
	const pending = $derived(!!$sourceImageData && shown.some((id) => !current[id]));
	const underSamples = $derived(underStage ? samples[underStage] : undefined);
	const overSamples = $derived(overStage ? samples[overStage] : undefined);
	const clip = $derived(
		overStage && current[overStage] && overSamples ? clipping(overSamples) : undefined
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
				() => shown,
				(next: string[]) => {
					// Keep one stage showing; an empty scope has nothing to compare.
					if (!next.length) return;
					recent ??= shown;
					const turnedOn = next.filter((id) => !shown.includes(id as Stage));
					recent = [
						...recent.filter((stage) => !turnedOn.includes(stage)),
						...(turnedOn as Stage[])
					];
					const keep = pickStages(next, recent);
					set({
						source: keep.includes('source'),
						effects: keep.includes('effects'),
						output: keep.includes('output')
					});
				}
			}
		>
			{#each STAGES as stage (stage.id)}
				<ToggleGroupItem value={stage.id} class="px-3 text-xs">{stage.label}</ToggleGroupItem>
			{/each}
		</ToggleGroup>
	</div>

	<div
		class="relative min-h-56 flex-1 border border-border bg-black text-white transition-opacity"
		class:opacity-60={pending}
		aria-busy={pending || undefined}
	>
		{#if !$sourceImageData}
			<p class="absolute inset-0 grid place-items-center text-xs text-white/50">
				Open an image to see its scopes.
			</p>
		{:else if scope.id === 'histogram'}
			<Histogram
				channels={model.channels}
				under={underSamples && histogram(underSamples, model.id)}
				over={overSamples && histogram(overSamples, model.id)}
			/>
		{:else if scope.id === 'waveform' || scope.id === 'parade'}
			<div class="size-full p-2">
				<Waveform
					channels={model.channels}
					parade={scope.id === 'parade'}
					under={underSamples && waveform(underSamples, model.id)}
					over={overSamples && waveform(overSamples, model.id)}
				/>
			</div>
		{:else if scope.id === 'vectorscope'}
			<div class="size-full p-2">
				<Vectorscope
					{plane}
					under={underSamples && vectorscope(underSamples, plane, settings.vectorZoom)}
					over={overSamples && vectorscope(overSamples, plane, settings.vectorZoom)}
					zoom={settings.vectorZoom}
					onzoom={(vectorZoom) => set({ vectorZoom })}
				/>
			</div>
		{:else}
			<div class="size-full p-2">
				<Chromaticity
					under={underSamples && chromaticityScope(underSamples)}
					over={overSamples && chromaticityScope(overSamples)}
				/>
			</div>
		{/if}
	</div>

	{#if $sourceImageData}
		<div class="flex h-4 items-center gap-2 text-xs tabular-nums">
			{#if scope.id === 'histogram'}
				{@const share = (value?: number) =>
					clip && value !== undefined ? formatShare(value) : '—'}
				{@const tone = (value?: number) =>
					clip && value ? 'text-foreground font-medium' : 'text-muted-foreground'}
				<span class={tone(clip?.black)}
					><span class="mr-1 inline-block size-2.5 border border-border bg-black"></span>Black
					{share(clip?.black)}</span
				>
				<span class="flex-1 text-center text-muted-foreground"
					>{pending
						? 'Updating…'
						: `${STAGES.find(({ id }) => id === overStage)?.label ?? ''} clipping`}</span
				>
				<span class={tone(clip?.white)}
					>White {share(clip?.white)}<span
						class="ml-1 inline-block size-2.5 border border-border bg-white"
					></span></span
				>
			{:else if pending}
				<span class="flex-1 text-center text-muted-foreground">Updating…</span>
			{/if}
		</div>
	{/if}
</div>
