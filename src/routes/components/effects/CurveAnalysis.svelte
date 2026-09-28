<script lang="ts">
	import type { Curve, CurvesEffect } from 'ditherette';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeft';
	import { Button } from '$lib/components/ui/button';
	import { Dialog, DialogContent, DialogTitle } from '$lib/components/ui/dialog';
	import { lutView } from '$lib/processing/lut-view';
	import { curveAnalysisOpen, sourceEffectsLut } from '$lib/processing/source-effects';
	import { sourceImageData, sourceObjectUrl } from '$lib/stores/app';
	import ColourScope3d from './ColourScope3d.svelte';
	import CurveSurface from './CurveSurface.svelte';
	import CurvesEditor from './CurvesEditor.svelte';

	type Props = {
		id: string;
		step: CurvesEffect;
		onchange: (step: CurvesEffect) => void;
		open: boolean;
		/** The selected curve, shared with the Curves window. */
		selected: number;
	};
	let { id, step, onchange, open = $bindable(), selected = $bindable() }: Props = $props();

	let wipe = $state(0.5);
	let viewer = $state<HTMLElement>();
	let wiping = false;

	const curve = $derived(step.curves[Math.min(selected, step.curves.length - 1)]);

	$effect(() => {
		curveAnalysisOpen.set(open);
		return () => curveAnalysisOpen.set(false);
	});

	function replace(next: Curve) {
		onchange({ ...step, curves: step.curves.map((c, index) => (c === curve ? next : c)) });
	}

	function setWipe(event: PointerEvent) {
		const box = viewer!.getBoundingClientRect();
		wipe = Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
	}
</script>

<Dialog bind:open>
	<DialogContent
		showCloseButton={false}
		class="top-0 left-0 grid h-dvh w-dvw max-w-none translate-x-0 translate-y-0 grid-rows-[auto_minmax(0,1fr)] gap-0 p-0 sm:max-w-none"
	>
		<header class="flex h-10 items-center gap-2 border-b border-border px-2">
			<Button variant="ghost" size="icon-sm" aria-label="Back" onclick={() => (open = false)}>
				<ArrowLeftIcon weight="bold" />
			</Button>
			<DialogTitle class="text-sm font-medium">Curves</DialogTitle>
		</header>
		<div class="grid min-h-0 grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_22rem] grid-rows-2">
			<section
				bind:this={viewer}
				class="relative col-start-1 row-span-2 row-start-1 min-h-0 touch-none overflow-hidden border-r border-border bg-[repeating-conic-gradient(theme(colors.muted)_0%_25%,transparent_0%_50%)_50%_/_16px_16px]"
				aria-label="Before and after"
				onpointerdown={(event) => {
					wiping = true;
					viewer!.setPointerCapture(event.pointerId);
					setWipe(event);
				}}
				onpointermove={(event) => wiping && setWipe(event)}
				onpointerup={() => (wiping = false)}
			>
				{#if $sourceObjectUrl}
					<img src={$sourceObjectUrl} alt="" class="absolute inset-0 size-full object-contain" />
				{/if}
				{#if $sourceImageData && $sourceEffectsLut}
					<canvas
						{@attach lutView}
						class="absolute inset-0 size-full object-contain"
						style:clip-path="inset(0 0 0 {wipe * 100}%)"
					></canvas>
				{/if}
				<div
					class="absolute inset-y-0 w-0.5 -translate-x-1/2 bg-primary"
					style:left="{wipe * 100}%"
				></div>
			</section>
			<section class="col-start-2 row-start-1 min-h-0 border-r border-b border-border">
				{#if curve}
					<CurveSurface {curve} onchange={replace} />
				{/if}
			</section>
			<section class="col-start-2 row-start-2 min-h-0 border-r border-border">
				<ColourScope3d source={$sourceImageData} lut={$sourceEffectsLut} />
			</section>
			<section class="col-start-3 row-span-2 row-start-1 min-h-0 overflow-y-auto p-3">
				<CurvesEditor {id} {step} {onchange} expanded bind:selected />
			</section>
		</div>
	</DialogContent>
</Dialog>
