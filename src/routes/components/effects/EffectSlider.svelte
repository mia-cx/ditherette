<script lang="ts">
	import { tick } from 'svelte';
	import { Label } from '$lib/components/ui/label';
	import { Slider } from '$lib/components/ui/slider';

	type Props = {
		id: string;
		label: string;
		value: number;
		min: number;
		max: number;
		step: number;
		unit?: string;
		/** Place the slider on a log scale, for ratios like gamma. */
		log?: boolean;
		/** A CSS gradient to draw as the track instead of the filled range. */
		track?: string;
		onchange: (value: number) => void;
	};
	let { id, label, value, min, max, step, unit, log = false, track, onchange }: Props = $props();

	const LOG_SLIDER_STEP = 0.005;
	const decimals = $derived(Math.max(0, -Math.floor(Math.log10(step))));
	const round = (next: number) => Number(next.toFixed(decimals));
	const clamp = (next: number) => Math.min(max, Math.max(min, next));
	const display = $derived(round(value));

	function slide(position: number) {
		onchange(clamp(round(log ? 10 ** position : position)));
	}

	async function type(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const next = Number(input.value);
		if (input.value.trim() === '' || !Number.isFinite(next)) {
			input.value = String(display);
			return;
		}
		onchange(clamp(round(next)));
		// The owner may clamp further, or keep the old value; show what it stored.
		await tick();
		input.value = String(display);
	}
</script>

<div class="grid grid-cols-[6rem_minmax(0,1fr)_5rem] items-center gap-2">
	<Label for={id} class="text-xs text-muted-foreground">{label}</Label>
	<Slider
		type="single"
		bind:value={() => (log ? Math.log10(value) : value), slide}
		min={log ? Math.log10(min) : min}
		max={log ? Math.log10(max) : max}
		step={log ? LOG_SLIDER_STEP : step}
		aria-label={label}
		class={track
			? '[&_[data-slot=slider-range]]:hidden [&_[data-slot=slider-track]]:h-2 [&_[data-slot=slider-track]]:bg-(image:--track)'
			: ''}
		style={track ? `--track: ${track}` : undefined}
	/>
	<div class="relative">
		<input
			{id}
			class="h-8 w-full border border-input bg-background px-2 text-right font-mono text-xs tabular-nums {unit
				? 'pr-6'
				: ''}"
			type="number"
			{min}
			{max}
			{step}
			value={display}
			onchange={type}
		/>
		{#if unit}
			<span
				class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-xs text-muted-foreground"
				>{unit}</span
			>
		{/if}
	</div>
</div>
