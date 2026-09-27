<script lang="ts">
	import type { Effect } from 'ditherette';
	import CurvesEditor from './CurvesEditor.svelte';
	import LevelsEditor from './LevelsEditor.svelte';
	import NumberFieldsEditor from './NumberFieldsEditor.svelte';
	import { NUMBER_FIELDS } from './fields';

	type Props = { id: string; step: Effect; onchange: (step: Effect) => void };
	let { id, step, onchange }: Props = $props();
</script>

{#if step.effect === 'levels'}
	<LevelsEditor {id} {step} {onchange} />
{:else if step.effect === 'curves'}
	<CurvesEditor {id} {step} {onchange} />
{:else if step.effect === 'brightness-contrast'}
	<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
{:else if step.effect === 'exposure'}
	<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
{:else if step.effect === 'white-balance'}
	<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
{:else if step.effect === 'hue-saturation'}
	<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
{:else}
	<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS[step.effect]} {onchange} />
{/if}
