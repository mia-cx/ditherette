<script lang="ts">
	import { onDestroy } from 'svelte';
	import { EFFECTS } from '$lib/effects/catalog';
	import { shownMask } from '$lib/processing/live-effects';
	import {
		effectLayers,
		removeEffect,
		renameEffect,
		setEffectEnabled,
		setEffectMask,
		updateEffect
	} from '$lib/stores/effects';
	import { Button } from '$lib/components/ui/button';
	import VisibilityCheckbox from '../VisibilityCheckbox.svelte';
	import EffectEditor from './EffectEditor.svelte';
	import MaskEditor from './MaskEditor.svelte';
	import ArrowCounterClockwiseIcon from 'phosphor-svelte/lib/ArrowCounterClockwise';
	import TrashIcon from 'phosphor-svelte/lib/Trash';

	type Props = {
		layerId: string;
		/** Show the name, enable toggle, and actions. Inline pipeline rows show their own. */
		header?: boolean;
	};
	let { layerId, header = true }: Props = $props();

	const layer = $derived($effectLayers.find((candidate) => candidate.id === layerId));

	function rename(event: Event & { currentTarget: HTMLInputElement }) {
		renameEffect(layerId, event.currentTarget.value);
		event.currentTarget.value = effectLayers.get().find((item) => item.id === layerId)?.name ?? '';
	}

	function reset() {
		if (!layer) return;
		updateEffect(layerId, { ...EFFECTS[layer.step.effect].create(), enabled: layer.step.enabled });
	}

	// Closing the controls stops showing their mask.
	onDestroy(() => {
		if (shownMask.get() === layerId) shownMask.set(undefined);
	});
</script>

{#if layer}
	<section class="grid grid-cols-1 gap-4" aria-label="{layer.name} controls">
		{#if header}
			<div class="flex items-center gap-2">
				<VisibilityCheckbox
					checked={layer.step.enabled}
					onCheckedChange={(enabled) => setEffectEnabled(layerId, enabled)}
					aria-label="Apply {layer.name}"
				/>
				<input
					class="h-8 min-w-0 flex-1 border border-transparent bg-transparent px-2 text-sm font-medium hover:border-input focus-visible:border-ring"
					aria-label="Effect name"
					value={layer.name}
					onchange={rename}
					onkeydown={(event) => {
						if (event.key === 'Enter') event.currentTarget.blur();
					}}
				/>
				<Button variant="ghost" size="icon-sm" aria-label="Reset {layer.name}" onclick={reset}>
					<ArrowCounterClockwiseIcon weight="bold" />
				</Button>
				<Button
					variant="ghost"
					size="icon-sm"
					aria-label="Remove {layer.name}"
					onclick={() => removeEffect(layerId)}
				>
					<TrashIcon weight="bold" />
				</Button>
			</div>
		{/if}
		<div class="grid grid-cols-1 gap-4 {layer.step.enabled ? '' : 'opacity-60'}">
			<EffectEditor
				id="effect-{layerId}"
				layerId={layer.id}
				step={layer.step}
				onchange={(step) => updateEffect(layerId, step)}
			/>
			<MaskEditor
				id="effect-{layerId}"
				mask={layer.step.mask ?? []}
				onchange={(mask) => setEffectMask(layerId, mask)}
				shown={$shownMask === layerId}
				onshow={(show) => shownMask.set(show ? layerId : undefined)}
			/>
		</div>
	</section>
{/if}
