<script lang="ts">
	type Props = {
		hueWheelBackground: string;
		hue: number;
		hueHandleStyle: string;
		triangleHandleStyle: string;
		onPick: (event: PointerEvent) => void;
	};

	let { hueWheelBackground, hue, hueHandleStyle, triangleHandleStyle, onPick }: Props = $props();
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
			class="absolute top-1/2 left-1/2 size-36 overflow-hidden"
			style="transform: translate(-50%, -50%) rotate({hue}deg);"
		>
			<svg class="absolute inset-0 size-full" viewBox="0 0 100 100" aria-hidden="true">
				<defs>
					<linearGradient
						id="wheel-triangle-white"
						gradientUnits="userSpaceOnUse"
						x1="25"
						y1="6.699"
						x2="62.5"
						y2="71.651"
					>
						<stop offset="0" stop-color="#fff" />
						<stop offset="1" stop-color="#fff" stop-opacity="0" />
					</linearGradient>
					<linearGradient
						id="wheel-triangle-black"
						gradientUnits="userSpaceOnUse"
						x1="25"
						y1="93.301"
						x2="62.5"
						y2="28.349"
					>
						<stop offset="0" stop-color="#000" />
						<stop offset="1" stop-color="#000" stop-opacity="0" />
					</linearGradient>
				</defs>
				<polygon points="100,50 25,6.699 25,93.301" fill="hsl({hue} 100% 50%)" />
				<polygon points="100,50 25,6.699 25,93.301" fill="url(#wheel-triangle-white)" />
				<polygon points="100,50 25,6.699 25,93.301" fill="url(#wheel-triangle-black)" />
			</svg>
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
