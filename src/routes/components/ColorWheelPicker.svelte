<script lang="ts">
	type Props = {
		hueWheelBackground: string;
		triangleBackground: string;
		hue: number;
		hueHandleStyle: string;
		triangleHandleStyle: string;
		onPick: (event: PointerEvent) => void;
	};

	let {
		hueWheelBackground,
		triangleBackground,
		hue,
		hueHandleStyle,
		triangleHandleStyle,
		onPick
	}: Props = $props();
</script>

<div class="grid place-items-center p-2">
	<button
		type="button"
		class="relative size-72 touch-none overflow-hidden rounded-full border-0 bg-transparent p-0"
		aria-label="Hue wheel with saturation and lightness triangle"
		onpointerdown={onPick}
	>
		<span class="absolute -inset-px rounded-full" style="background: {hueWheelBackground};"></span>
		<span class="absolute inset-[13%] rounded-full bg-background"></span>
		<span
			class="absolute top-1/2 left-1/2 size-36 overflow-hidden [clip-path:polygon(100%_50%,25%_6.699%,25%_93.301%)]"
			style="transform: translate(-50%, -50%) rotate({hue}deg);"
		>
			<span class="absolute -inset-px" style="background: {triangleBackground};"></span>
		</span>
		<span
			class="absolute size-4 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={hueHandleStyle}
		></span>
		<span
			class="absolute size-3 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={triangleHandleStyle}
		></span>
	</button>
</div>
