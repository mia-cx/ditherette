<script lang="ts">
	import type { EffectChannel } from 'ditherette';
	import { ToggleGroup, ToggleGroupItem } from '$lib/components/ui/toggle-group';

	type Props = { value: EffectChannel; onchange: (channel: EffectChannel) => void };
	let { value, onchange }: Props = $props();

	const CHANNELS = [
		{ id: 'rgb', label: 'RGB' },
		{ id: 'red', label: 'Red' },
		{ id: 'green', label: 'Green' },
		{ id: 'blue', label: 'Blue' }
	] as const satisfies readonly { id: EffectChannel; label: string }[];
</script>

<ToggleGroup
	type="single"
	variant="outline"
	size="sm"
	bind:value={
		() => value,
		(next) => {
			// Clicking the selected channel would clear it; keep one selected.
			const channel = CHANNELS.find((option) => option.id === next);
			if (channel) onchange(channel.id);
		}
	}
	aria-label="Channel"
	class="w-full"
>
	{#each CHANNELS as channel (channel.id)}
		<ToggleGroupItem value={channel.id} class="flex-1 text-xs">{channel.label}</ToggleGroupItem>
	{/each}
</ToggleGroup>
