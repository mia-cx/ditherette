<script lang="ts">
	import type { LevelsEffect } from 'ditherette';
	import ChannelToggle from './ChannelToggle.svelte';
	import EffectSlider from './EffectSlider.svelte';

	type Props = { id: string; step: LevelsEffect; onchange: (step: LevelsEffect) => void };
	let { id, step, onchange }: Props = $props();

	/** Levels reads in byte units, like every editor's Levels panel. */
	const BYTE = 255;
	const toByte = (value: number) => value * BYTE;
	const fromByte = (value: number) => value / BYTE;

	// Input black must stay below input white; output black above white inverts, so it may cross.
	function setInput(edge: 'black' | 'white', byte: number) {
		const other = toByte(edge === 'black' ? step.input.white : step.input.black);
		const clamped = edge === 'black' ? Math.min(byte, other - 1) : Math.max(byte, other + 1);
		onchange({ ...step, input: { ...step.input, [edge]: fromByte(clamped) } });
	}
</script>

<div class="grid grid-cols-1 gap-3">
	<ChannelToggle value={step.channel} onchange={(channel) => onchange({ ...step, channel })} />
	<EffectSlider
		id="{id}-input-black"
		label="Input black"
		value={toByte(step.input.black)}
		min={0}
		max={BYTE - 1}
		step={1}
		onchange={(value) => setInput('black', value)}
	/>
	<EffectSlider
		id="{id}-input-white"
		label="Input white"
		value={toByte(step.input.white)}
		min={1}
		max={BYTE}
		step={1}
		onchange={(value) => setInput('white', value)}
	/>
	<EffectSlider
		id="{id}-gamma"
		label="Midtones"
		value={step.gamma}
		min={0.1}
		max={10}
		step={0.01}
		log
		onchange={(gamma) => onchange({ ...step, gamma })}
	/>
	<EffectSlider
		id="{id}-output-black"
		label="Output black"
		value={toByte(step.output.black)}
		min={0}
		max={BYTE}
		step={1}
		onchange={(value) => onchange({ ...step, output: { ...step.output, black: fromByte(value) } })}
	/>
	<EffectSlider
		id="{id}-output-white"
		label="Output white"
		value={toByte(step.output.white)}
		min={0}
		max={BYTE}
		step={1}
		onchange={(value) => onchange({ ...step, output: { ...step.output, white: fromByte(value) } })}
	/>
</div>
