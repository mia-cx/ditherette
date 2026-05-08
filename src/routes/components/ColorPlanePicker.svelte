<script lang="ts">
	import { onDestroy } from 'svelte';

	type PlanePoint = { x: number; y: number };
	type Props = {
		fieldBackground: string;
		handleStyle: string;
		handlePoint: PlanePoint;
		onPickPlane: (point: PlanePoint) => void;
	};

	let { fieldBackground, handleStyle, handlePoint, onPickPlane }: Props = $props();
	let pendingPoint: PlanePoint | null = null;
	let commitFrame: number | null = null;
	let activeDragCleanup: (() => void) | null = null;

	onDestroy(() => {
		activeDragCleanup?.();
		if (commitFrame === null) return;
		cancelAnimationFrame(commitFrame);
		commitFrame = null;
		pendingPoint = null;
	});

	function pickPlane(event: PointerEvent) {
		activeDragCleanup?.();
		const target = event.currentTarget as HTMLElement;
		const handle = target.querySelector<HTMLElement>('[data-plane-handle]');
		const update = (next: PointerEvent) => updateHandle(handle, pointFromPointer(next, target));
		const pointerId = event.pointerId;
		let cleanedUp = false;
		const cleanup = (next?: PointerEvent) => {
			if (next && next.pointerId !== pointerId) return;
			if (cleanedUp) return;
			cleanedUp = true;
			window.removeEventListener('pointerup', cleanup);
			window.removeEventListener('pointercancel', cleanup);
			cleanupPointer(target, pointerId);
			activeDragCleanup = null;
		};
		activeDragCleanup = () => cleanup();
		target.setPointerCapture(pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = cleanup;
		target.onpointercancel = cleanup;
		target.onlostpointercapture = cleanup;
		window.addEventListener('pointerup', cleanup);
		window.addEventListener('pointercancel', cleanup);
	}

	function handlePlaneKeydown(event: KeyboardEvent) {
		const deltas: Record<string, PlanePoint> = {
			ArrowLeft: { x: -1, y: 0 },
			ArrowRight: { x: 1, y: 0 },
			ArrowUp: { x: 0, y: -1 },
			ArrowDown: { x: 0, y: 1 }
		};
		const delta = deltas[event.key];
		if (!delta) return;
		event.preventDefault();
		const step = event.shiftKey ? 0.1 : 0.01;
		const target = event.currentTarget as HTMLElement;
		const handle = target.querySelector<HTMLElement>('[data-plane-handle]');
		updateHandle(handle, {
			x: clamp(handlePoint.x + delta.x * step, 0, 1),
			y: clamp(handlePoint.y + delta.y * step, 0, 1)
		});
	}

	function updateHandle(handle: HTMLElement | null, point: PlanePoint) {
		handle?.style.setProperty('left', `${point.x * 100}%`);
		handle?.style.setProperty('top', `${point.y * 100}%`);
		scheduleCommit(point);
	}

	function pointFromPointer(event: PointerEvent, target: HTMLElement) {
		const rect = target.getBoundingClientRect();
		return {
			x: rect.width === 0 ? 0 : clamp((event.clientX - rect.left) / rect.width, 0, 1),
			y: rect.height === 0 ? 0 : clamp((event.clientY - rect.top) / rect.height, 0, 1)
		};
	}

	function cleanupPointer(target: HTMLElement, pointerId: number) {
		if (target.hasPointerCapture(pointerId)) target.releasePointerCapture(pointerId);
		target.onpointermove = null;
		target.onpointerup = null;
		target.onpointercancel = null;
		target.onlostpointercapture = null;
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

	function clamp(value: number, min: number, max: number) {
		return Math.min(max, Math.max(min, value));
	}
</script>

<div class="grid">
	<button
		type="button"
		class="relative aspect-square min-h-52 touch-none overflow-hidden border border-border bg-transparent p-0"
		aria-label="Color field. Arrow keys move the color stop."
		onpointerdown={pickPlane}
		onkeydown={handlePlaneKeydown}
	>
		<span class="absolute -inset-px" style="background: {fieldBackground};"></span>
		<span
			data-plane-handle
			class="absolute size-4 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={handleStyle}
		></span>
	</button>
</div>
