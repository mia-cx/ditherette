<script lang="ts">
	import { tick } from 'svelte';
	import { ADDABLE_KINDS, EFFECTS, type EffectKind } from '$lib/effects/catalog';
	import {
		addEffect,
		effectLayers,
		effectStepsLeft,
		moveEffect,
		removeEffect,
		renameEffect,
		setEffectEnabled
	} from '$lib/stores/effects';
	import { Button } from '$lib/components/ui/button';
	import {
		DropdownMenu,
		DropdownMenuContent,
		DropdownMenuItem,
		DropdownMenuSeparator,
		DropdownMenuTrigger
	} from '$lib/components/ui/dropdown-menu';
	import { Badge } from '$lib/components/ui/badge';
	import VisibilityCheckbox from '../VisibilityCheckbox.svelte';
	import EffectControls from './EffectControls.svelte';
	import CaretDownIcon from 'phosphor-svelte/lib/CaretDown';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRight';
	import DotsSixVerticalIcon from 'phosphor-svelte/lib/DotsSixVertical';
	import DotsThreeIcon from 'phosphor-svelte/lib/DotsThree';
	import PlusIcon from 'phosphor-svelte/lib/Plus';

	type Props = {
		/** Open a layer's own window. Without it, rows expand to show their controls inline. */
		onOpen?: (layerId: string) => void;
	};
	let { onOpen }: Props = $props();

	let expanded = $state<string[]>([]);
	let renaming = $state<string>();
	let dragged = $state<string>();
	let dropIndex = $state<number>();

	function add(kind: EffectKind) {
		const layer = addEffect(kind);
		if (onOpen) onOpen(layer.id);
		else expanded = [...expanded, layer.id];
	}

	function activate(layerId: string) {
		if (onOpen) return onOpen(layerId);
		expanded = expanded.includes(layerId)
			? expanded.filter((id) => id !== layerId)
			: [...expanded, layerId];
	}

	async function startRename(layerId: string) {
		renaming = layerId;
		await tick();
		document.querySelector<HTMLInputElement>(`[data-rename="${layerId}"]`)?.select();
	}

	function finishRename(layerId: string, input: HTMLInputElement, save: boolean) {
		if (renaming !== layerId) return;
		if (save) renameEffect(layerId, input.value);
		renaming = undefined;
	}

	function drop(index: number) {
		if (dragged) {
			const from = $effectLayers.findIndex((layer) => layer.id === dragged);
			moveEffect(dragged, from < index ? index - 1 : index);
		}
		dragged = undefined;
		dropIndex = undefined;
	}

	function dragOver(event: DragEvent, index: number) {
		if (!dragged) return;
		event.preventDefault();
		const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
		dropIndex = event.clientY < box.top + box.height / 2 ? index : index + 1;
	}
</script>

<section class="flex min-h-0 flex-col gap-2" aria-label="Effect pipeline">
	<DropdownMenu>
		<DropdownMenuTrigger>
			{#snippet child({ props })}
				<Button
					{...props}
					variant="outline"
					size="sm"
					class="w-full justify-start"
					disabled={$effectStepsLeft < 1}
				>
					<PlusIcon weight="bold" />
					Add effect
				</Button>
			{/snippet}
		</DropdownMenuTrigger>
		<DropdownMenuContent align="start" class="w-(--bits-dropdown-menu-anchor-width)">
			{#each ADDABLE_KINDS as kind (kind)}
				<DropdownMenuItem disabled={$effectStepsLeft < 1} onSelect={() => add(kind)}
					>{EFFECTS[kind].label}</DropdownMenuItem
				>
			{/each}
		</DropdownMenuContent>
	</DropdownMenu>

	{#if $effectLayers.length}
		<ol class="grid min-h-0 content-start" ondragleave={() => (dropIndex = undefined)}>
			{#each $effectLayers as layer, index (layer.id)}
				{@const open = !onOpen && expanded.includes(layer.id)}
				{@const label = EFFECTS[layer.step.effect].label}
				<li
					class="border-t-2 {dropIndex === index
						? 'border-t-primary'
						: 'border-t-transparent'} {index === $effectLayers.length - 1 && dropIndex === index + 1
						? 'border-b-2 border-b-primary'
						: ''}"
					ondragover={(event) => dragOver(event, index)}
					ondrop={() => drop(dropIndex ?? index)}
				>
					<div
						class="group flex h-9 items-center gap-1.5 pr-1 hover:bg-muted/60 {dragged === layer.id
							? 'opacity-50'
							: ''}"
					>
						<span
							class="flex h-full cursor-grab items-center text-muted-foreground"
							draggable={renaming !== layer.id}
							role="presentation"
							ondragstart={(event) => {
								dragged = layer.id;
								event.dataTransfer?.setData('text/plain', layer.name);
							}}
							ondragend={() => {
								dragged = undefined;
								dropIndex = undefined;
							}}
						>
							<DotsSixVerticalIcon weight="bold" class="size-4" />
						</span>
						<VisibilityCheckbox
							checked={layer.step.enabled}
							onCheckedChange={(enabled) => setEffectEnabled(layer.id, enabled)}
							aria-label="Apply {layer.name}"
						/>
						{#if renaming === layer.id}
							<input
								data-rename={layer.id}
								class="h-7 min-w-0 flex-1 border border-ring bg-background px-2 text-sm"
								aria-label="Effect name"
								value={layer.name}
								onblur={(event) => finishRename(layer.id, event.currentTarget, true)}
								onkeydown={(event) => {
									if (event.key === 'Enter') finishRename(layer.id, event.currentTarget, true);
									if (event.key === 'Escape') finishRename(layer.id, event.currentTarget, false);
								}}
							/>
						{:else}
							<button
								type="button"
								class="flex h-full min-w-0 flex-1 items-center gap-1.5 px-1 text-left text-sm outline-none focus-visible:ring-1 focus-visible:ring-ring {layer
									.step.enabled
									? ''
									: 'text-muted-foreground'}"
								aria-expanded={onOpen ? undefined : open}
								onclick={() => activate(layer.id)}
								ondblclick={() => startRename(layer.id)}
							>
								{#if !onOpen}
									{#if open}
										<CaretDownIcon weight="bold" class="size-3 shrink-0" />
									{:else}
										<CaretRightIcon weight="bold" class="size-3 shrink-0" />
									{/if}
								{/if}
								<span class="truncate">{layer.name}</span>
								{#if layer.step.effect === 'palette-fit' && layer.step.curves !== null}
									<Badge variant="secondary" class="shrink-0">Edited</Badge>
								{/if}
								{#if layer.name !== label && !layer.name.startsWith(label)}
									<span class="truncate text-xs text-muted-foreground">{label}</span>
								{/if}
							</button>
						{/if}
						<DropdownMenu>
							<DropdownMenuTrigger>
								{#snippet child({ props })}
									<Button
										{...props}
										variant="ghost"
										size="icon-xs"
										aria-label="{layer.name} actions"
									>
										<DotsThreeIcon weight="bold" />
									</Button>
								{/snippet}
							</DropdownMenuTrigger>
							<DropdownMenuContent align="end">
								{#if onOpen}
									<DropdownMenuItem onSelect={() => onOpen(layer.id)}>Open window</DropdownMenuItem>
								{/if}
								<DropdownMenuItem onSelect={() => startRename(layer.id)}>Rename</DropdownMenuItem>
								<DropdownMenuItem
									disabled={index === 0}
									onSelect={() => moveEffect(layer.id, index - 1)}>Move up</DropdownMenuItem
								>
								<DropdownMenuItem
									disabled={index === $effectLayers.length - 1}
									onSelect={() => moveEffect(layer.id, index + 1)}>Move down</DropdownMenuItem
								>
								<DropdownMenuSeparator />
								<DropdownMenuItem variant="destructive" onSelect={() => removeEffect(layer.id)}
									>Remove</DropdownMenuItem
								>
							</DropdownMenuContent>
						</DropdownMenu>
					</div>
					{#if open}
						<div class="px-2 pt-2 pb-4">
							<EffectControls layerId={layer.id} header={false} />
						</div>
					{/if}
				</li>
			{/each}
		</ol>
	{:else}
		<p class="px-1 py-2 text-sm text-muted-foreground">No effects yet.</p>
	{/if}
</section>
