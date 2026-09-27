<script lang="ts" generics="S extends Effect">
	import type { Effect } from 'ditherette';
	import EffectSlider from './EffectSlider.svelte';
	import type { NumberField } from './fields';

	type Props = {
		id: string;
		step: S;
		fields: readonly NumberField<S>[];
		onchange: (step: S) => void;
	};
	let { id, step, fields, onchange }: Props = $props();
</script>

<div class="grid grid-cols-1 gap-3">
	{#each fields as field (field.key)}
		{@const scale = field.scale ?? 1}
		<EffectSlider
			id="{id}-{String(field.key)}"
			label={field.label}
			value={(step[field.key] as number) * scale}
			min={field.min}
			max={field.max}
			step={field.step}
			unit={field.unit}
			track={field.track}
			onchange={(value) => onchange({ ...step, [field.key]: value / scale })}
		/>
	{/each}
</div>
