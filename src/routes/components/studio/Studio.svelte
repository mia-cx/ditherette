<script lang="ts">
	import type { DockviewApi } from 'dockview-core';
	import Dock from '$lib/components/dock/Dock.svelte';
	import { Button } from '$lib/components/ui/button';
	import { hasImage } from '$lib/stores/app';
	import { effectLayers } from '$lib/stores/effects';
	import ColorSpacePanel from '../ColorSpacePanel.svelte';
	import ComparisonPreview from '../ComparisonPreview.svelte';
	import DitherPanel from '../DitherPanel.svelte';
	import OutputPanel from '../OutputPanel.svelte';
	import PalettePanel from '../PalettePanel.svelte';
	import EffectControls from '../effects/EffectControls.svelte';
	import PipelinePanel from '../effects/PipelinePanel.svelte';
	import ScopesPanel from '../scopes/ScopesPanel.svelte';
	import {
		EFFECT_COMPONENT,
		openEffectWindow,
		resetLayout,
		startStudio,
		syncEffectWindows
	} from './workspace';

	type Props = { onChooseImage: () => void; onSelectFile: (file: File) => void };
	let { onChooseImage, onSelectFile }: Props = $props();

	function ready(api: DockviewApi) {
		const stopStudio = startStudio(api);
		const unsubscribe = effectLayers.subscribe((layers) => syncEffectWindows(api, layers));
		return () => {
			unsubscribe();
			stopStudio();
		};
	}
</script>

<Dock
	main="preview"
	panels={{
		preview,
		effects,
		dimensions,
		dither,
		'color-space': colorSpace,
		palette,
		scopes,
		[EFFECT_COMPONENT]: effect
	}}
	{empty}
	onready={ready}
/>

{#snippet preview()}
	<ComparisonPreview hasImage={$hasImage} minHeightClass="h-full" {onChooseImage} {onSelectFile} />
{/snippet}

{#snippet effects()}
	<div class="p-3"><PipelinePanel onOpen={openEffectWindow} /></div>
{/snippet}

{#snippet dimensions()}
	<div class="p-4"><OutputPanel hasImage={$hasImage} hideHeading /></div>
{/snippet}

{#snippet dither()}
	<div class="p-4"><DitherPanel hideHeading /></div>
{/snippet}

{#snippet colorSpace()}
	<div class="p-4"><ColorSpacePanel hideHeading /></div>
{/snippet}

{#snippet palette()}
	<div class="flex h-full min-h-0 flex-col p-3"><PalettePanel fillHeight hideHeading /></div>
{/snippet}

{#snippet scopes()}
	<ScopesPanel />
{/snippet}

{#snippet effect(params: Readonly<Record<string, string>>)}
	<div class="p-3"><EffectControls layerId={params.layerId ?? ''} /></div>
{/snippet}

{#snippet empty()}
	<div class="grid h-full place-items-center">
		<Button variant="outline" onclick={resetLayout}>Reset layout</Button>
	</div>
{/snippet}
