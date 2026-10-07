<script lang="ts">
	import type { Curve, CurvesEffect, PaletteFitEffect, PaletteFitSpace } from 'ditherette';
	import {
		AlertDialog,
		AlertDialogAction,
		AlertDialogCancel,
		AlertDialogContent,
		AlertDialogDescription,
		AlertDialogFooter,
		AlertDialogHeader,
		AlertDialogTitle
	} from '$lib/components/ui/alert-dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '$lib/components/ui/collapsible';
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import CaretRightIcon from 'phosphor-svelte/lib/CaretRight';
	import { FIT_SPACES } from '$lib/effects/catalog';
	import { analysedFits } from '$lib/processing/live-effects';
	import { fitReverts, revertFit } from '$lib/stores/fit-revert';
	import CurvesEditor from './CurvesEditor.svelte';
	import NumberFieldsEditor from './NumberFieldsEditor.svelte';
	import { NUMBER_FIELDS } from './fields';

	type Props = {
		id: string;
		layerId: string;
		step: PaletteFitEffect;
		onchange: (step: PaletteFitEffect) => void;
	};
	let { id, layerId, step, onchange }: Props = $props();

	let expanded = $state(false);
	/** A pending space switch that would discard edited curves. */
	let confirmSpace = $state<PaletteFitSpace | undefined>();

	const spaceLabel = (space: PaletteFitSpace) =>
		FIT_SPACES.find((option) => option.id === space)!.label;

	/** The step's curves while edited, else the analysis the worker last resolved. */
	const resolved = $derived(step.curves ?? $analysedFits.get(layerId));
	const curvesStep = $derived<CurvesEffect>({
		effect: 'curves',
		enabled: true,
		curves: resolved ?? []
	});

	/** Curves canvas changes always edit the step's own list: the first edit locks the fit. */
	function editCurves(next: CurvesEffect) {
		onchange({ ...step, curves: [...next.curves] });
	}

	function chooseSpace(space: string) {
		if (space === step.space) return;
		if (step.curves === null) {
			onchange({ ...step, space: space as PaletteFitSpace });
		} else {
			confirmSpace = space as PaletteFitSpace;
		}
	}

	function reanalyse() {
		const space = confirmSpace;
		confirmSpace = undefined;
		if (space) onchange({ ...step, space, curves: null });
	}
</script>

<div class="grid grid-cols-1 gap-4">
	<NumberFieldsEditor {id} {step} fields={NUMBER_FIELDS['palette-fit']} {onchange} />
	<div class="grid gap-1.5">
		<Label for="{id}-space">Space</Label>
		<Select type="single" value={step.space} onValueChange={chooseSpace}>
			<SelectTrigger id="{id}-space" class="w-full">{spaceLabel(step.space)}</SelectTrigger>
			<SelectContent>
				{#each FIT_SPACES as space (space.id)}
					<SelectItem value={space.id}>{space.label}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
	</div>

	{#if $fitReverts.has(layerId)}
		<div class="flex items-center justify-between gap-2 rounded-md border border-border p-2">
			<span class="text-xs text-muted-foreground">Re-analysed for the new inputs.</span>
			<Button variant="outline" size="sm" onclick={() => revertFit(layerId)}>Revert</Button>
		</div>
	{/if}

	<Collapsible bind:open={expanded} class="grid gap-2">
		<div class="flex items-center gap-2">
			<CollapsibleTrigger
				class="flex h-8 flex-1 items-center gap-1.5 text-left text-sm font-medium"
				aria-label="Advanced"
			>
				<CaretRightIcon
					weight="bold"
					class="size-3 shrink-0 transition-transform {expanded ? 'rotate-90' : ''}"
				/>
				Advanced
			</CollapsibleTrigger>
			{#if step.curves !== null}
				<Badge variant="secondary">Edited</Badge>
				<Button
					variant="outline"
					size="sm"
					onclick={() => onchange({ ...step, curves: null })}
				>
					Reset
				</Button>
			{/if}
		</div>
		<CollapsibleContent>
			{#if resolved}
				<CurvesEditor {id} step={curvesStep} onchange={editCurves} />
			{:else}
				<p class="text-xs text-muted-foreground">Analysing the fit…</p>
			{/if}
		</CollapsibleContent>
	</Collapsible>

	<AlertDialog open={confirmSpace !== undefined}>
		<AlertDialogContent>
			<AlertDialogHeader>
				<AlertDialogTitle
					>Re-analyse in {confirmSpace ? spaceLabel(confirmSpace) : ''}?</AlertDialogTitle
				>
				<AlertDialogDescription>Your curve edits will be lost.</AlertDialogDescription>
			</AlertDialogHeader>
			<AlertDialogFooter>
				<AlertDialogCancel onclick={() => (confirmSpace = undefined)}>Cancel</AlertDialogCancel>
				<AlertDialogAction onclick={reanalyse}>Re-analyse</AlertDialogAction>
			</AlertDialogFooter>
		</AlertDialogContent>
	</AlertDialog>
</div>
