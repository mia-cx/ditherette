<script lang="ts">
	import { onDestroy } from 'svelte';

	type PlanePoint = { x: number; y: number };
	type Props = {
		fieldBackground: string;
		handleStyle: string;
		onPickPlane: (point: PlanePoint) => void;
	};

	let { fieldBackground, handleStyle, onPickPlane }: Props = $props();
	let pendingPoint: PlanePoint | null = null;
	let commitFrame: number | null = null;

	onDestroy(() => {
		if (commitFrame === null) return;
		cancelAnimationFrame(commitFrame);
		commitFrame = null;
		pendingPoint = null;
	});

	function pickPlane(event: PointerEvent) {
		const target = event.currentTarget as HTMLElement;
		const handle = target.querySelector<HTMLElement>('[data-plane-handle]');
		const update = (next: PointerEvent) => {
			const rect = target.getBoundingClientRect();
			const point = {
				x: Math.min(1, Math.max(0, (next.clientX - rect.left) / rect.width)),
				y: Math.min(1, Math.max(0, (next.clientY - rect.top) / rect.height))
			};
			handle?.style.setProperty('left', `${point.x * 100}%`);
			handle?.style.setProperty('top', `${point.y * 100}%`);
			scheduleCommit(point);
		};
		target.setPointerCapture(event.pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = () => (target.onpointermove = null);
		target.onpointercancel = () => (target.onpointermove = null);
	}

	function scheduleCommit(point: PlanePoint) {
		pendingPoint = point;
		if (commitFrame !== null) return;
		commitFrame = requestAnimationFrame(() => {
			commitFrame = null;
			const point = pendingPoint;
			pendingPoint = null;
			if (point) onPickPlane(point);
		});
	}
</script>

<div class="grid">
	<button
		type="button"
		class="relative aspect-square min-h-52 touch-none overflow-hidden border border-border bg-transparent p-0"
		aria-label="Color field"
		onpointerdown={pickPlane}
	>
		<span class="absolute -inset-px" style="background: {fieldBackground};"></span>
		<span
			data-plane-handle
			class="absolute size-4 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={handleStyle}
		></span>
	</button>
</div>
