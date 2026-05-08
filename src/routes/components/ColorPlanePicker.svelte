<script lang="ts">
	type PlanePoint = { x: number; y: number };
	type Props = {
		fieldBackground: string;
		handleStyle: string;
		onPickPlane: (point: PlanePoint) => void;
	};

	let { fieldBackground, handleStyle, onPickPlane }: Props = $props();
	let dragHandle = $state<PlanePoint | null>(null);

	const displayHandleStyle = $derived(
		dragHandle ? `left: ${dragHandle.x * 100}%; top: ${dragHandle.y * 100}%;` : handleStyle
	);

	function pickPlane(event: PointerEvent) {
		const target = event.currentTarget as HTMLElement;
		const update = (next: PointerEvent) => {
			const rect = target.getBoundingClientRect();
			const point = {
				x: Math.min(1, Math.max(0, (next.clientX - rect.left) / rect.width)),
				y: Math.min(1, Math.max(0, (next.clientY - rect.top) / rect.height))
			};
			dragHandle = point;
			onPickPlane(point);
		};
		target.setPointerCapture(event.pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = () => {
			target.onpointermove = null;
			queueMicrotask(() => (dragHandle = null));
		};
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
			class="absolute size-4 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={displayHandleStyle}
		></span>
	</button>
</div>
