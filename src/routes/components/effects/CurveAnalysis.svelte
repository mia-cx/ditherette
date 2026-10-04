<script lang="ts">
	import type { Curve, CurvesEffect } from 'ditherette';
	import ArrowLeftIcon from 'phosphor-svelte/lib/ArrowLeft';
	import { Button } from '$lib/components/ui/button';
	import { Dialog, DialogContent, DialogTitle } from '$lib/components/ui/dialog';
	import { lutView, shownEffectsTable } from '$lib/processing/lut-view';
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

	const active = $derived(Math.min(selected, step.curves.length - 1));
	const curve = $derived(step.curves[active]);

	function replace(next: Curve) {
		onchange({
			...step,
			curves: step.curves.map((curve, index) => (index === active ? next : curve))
		});
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
		<div
			class="grid min-h-0 grid-cols-1 grid-rows-[minmax(16rem,45dvh)_16rem_16rem_auto] overflow-y-auto lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_22rem] lg:grid-rows-2 lg:overflow-hidden"
		>
			<section
				bind:this={viewer}
				class="relative min-h-0 min-w-0 touch-none overflow-hidden border-b border-border bg-[repeating-conic-gradient(theme(colors.muted)_0%_25%,transparent_0%_50%)_50%_/_16px_16px] lg:col-start-1 lg:row-span-2 lg:row-start-1 lg:border-r lg:border-b-0"
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
				{#if $sourceImageData && $shownEffectsTable}
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
			<section
				class="min-h-0 min-w-0 border-b border-border lg:col-start-2 lg:row-start-1 lg:border-r"
			>
				{#if curve}
					<CurveSurface {curve} onchange={replace} />
				{/if}
			</section>
			<section
				class="min-h-0 min-w-0 border-b border-border lg:col-start-2 lg:row-start-2 lg:border-r lg:border-b-0"
			>
				<ColourScope3d source={$sourceImageData} table={$shownEffectsTable} />
			</section>
			<section
				class="min-h-0 min-w-0 p-3 lg:col-start-3 lg:row-span-2 lg:row-start-1 lg:overflow-y-auto"
			>
				<CurvesEditor {id} {step} {onchange} expanded bind:selected />
			</section>
		</div>
	</DialogContent>
</Dialog>
