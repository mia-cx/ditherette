<script lang="ts">
	import type { Effect } from 'ditherette';
	import { brightnessContrastTone, exposureTone, hueSpectrum, levelsTone } from '$lib/effects/tone';
	import CurvesEditor from './CurvesEditor.svelte';
	import LevelsEditor from './LevelsEditor.svelte';
	import NumberFieldsEditor from './NumberFieldsEditor.svelte';
	import ToneGraph from './ToneGraph.svelte';
	import { NUMBER_FIELDS } from './fields';

	type Props = { id: string; step: Effect; onchange: (step: Effect) => void };
	let { id, step, onchange }: Props = $props();

	const LEVELS_STROKE = {
		rgb: 'text-foreground',
		red: 'text-red-500',
		green: 'text-green-500',
		blue: 'text-blue-500'
	} as const;
</script>

<div class="grid grid-cols-1 gap-4">
	{#if step.effect === 'levels'}
		<ToneGraph
			label="Levels response"
			curves={[{ map: levelsTone(step), class: LEVELS_STROKE[step.channel] }]}
		/>
		<LevelsEditor {id} {step} {onchange} />
	{:else if step.effect === 'curves'}
		<CurvesEditor {id} {step} {onchange} />
	{:else if step.effect === 'brightness-contrast'}
		<ToneGraph
			label="Brightness and contrast response"
			curves={[{ map: brightnessContrastTone(step), class: 'text-foreground' }]}
		/>
		<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
	{:else if step.effect === 'exposure'}
		<ToneGraph
			label="Exposure response"
			curves={[{ map: exposureTone(step), class: 'text-foreground' }]}
		/>
		<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
	{:else if step.effect === 'white-balance'}
		<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
	{:else if step.effect === 'hue-saturation'}
		<!-- Input hues above, the same hues after this step below. -->
		<div class="grid gap-1" aria-hidden="true">
			<div class="h-3 border border-border" style:background-image={hueSpectrum({})}></div>
			<div class="h-3 border border-border" style:background-image={hueSpectrum({ step })}></div>
		</div>
		<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
	{:else}
		<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
	{/if}
</div>
