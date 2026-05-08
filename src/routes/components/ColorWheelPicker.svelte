<script lang="ts">
	type Point = { x: number; y: number };
	type Props = {
		hueWheelBackground: string;
		hue: number;
		hueHandleStyle: string;
		triangleHandleStyle: string;
		onPickHue: (hue: number) => void;
		onPickTriangle: (point: Point, width: number) => void;
	};

	let {
		hueWheelBackground,
		hue,
		hueHandleStyle,
		triangleHandleStyle,
		onPickHue,
		onPickTriangle
	}: Props = $props();
	let dragHueHandleStyle = $state<string | null>(null);
	let dragTriangleHandleStyle = $state<string | null>(null);

	const displayHueHandleStyle = $derived(dragHueHandleStyle ?? hueHandleStyle);
	const displayTriangleHandleStyle = $derived(dragTriangleHandleStyle ?? triangleHandleStyle);

	function pickWheel(event: PointerEvent) {
		const target = event.currentTarget as HTMLElement;
		const dragMode = wheelDragMode(event, target);
		const update = (next: PointerEvent) => {
			const rect = target.getBoundingClientRect();
			const point = wheelPoint(next, rect);
			if (dragMode === 'hue') {
				const nextHue = hueFromWheelPoint(point.x, point.y);
				dragHueHandleStyle = wheelHandleStyle(nextHue);
				onPickHue(nextHue);
				return;
			}
			const percent = {
				x: Math.min(100, Math.max(0, ((point.x + rect.width / 2) / rect.width) * 100)),
				y: Math.min(100, Math.max(0, ((point.y + rect.height / 2) / rect.height) * 100))
			};
			dragTriangleHandleStyle = `left: ${percent.x}%; top: ${percent.y}%;`;
			onPickTriangle(point, rect.width);
		};
		target.setPointerCapture(event.pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = () => {
			target.onpointermove = null;
			queueMicrotask(() => {
				dragHueHandleStyle = null;
				dragTriangleHandleStyle = null;
			});
		};
	}

	function wheelDragMode(event: PointerEvent, target: HTMLElement): 'hue' | 'triangle' {
		const rect = target.getBoundingClientRect();
		const point = wheelPoint(event, rect);
		return Math.hypot(point.x, point.y) >= (rect.width / 2) * 0.74 ? 'hue' : 'triangle';
	}

	function wheelPoint(event: PointerEvent, rect: DOMRect) {
		return {
			x: event.clientX - (rect.left + rect.width / 2),
			y: event.clientY - (rect.top + rect.height / 2)
		};
	}

	function hueFromWheelPoint(x: number, y: number) {
		return ((Math.atan2(y, x) * 180) / Math.PI + 360) % 360;
	}

	function wheelHandleStyle(nextHue: number) {
		const angle = (nextHue * Math.PI) / 180;
		return `left: ${50 + Math.cos(angle) * 43}%; top: ${50 + Math.sin(angle) * 43}%;`;
	}
</script>

<div class="grid place-items-center p-2">
	<button
		type="button"
		class="relative size-72 touch-none overflow-hidden rounded-full border-0 bg-transparent p-0"
		aria-label="Hue wheel with saturation and lightness triangle"
		onpointerdown={pickWheel}
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
			style={displayHueHandleStyle}
		></span>
		<span
			class="absolute size-3 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={displayTriangleHandleStyle}
		></span>
	</button>
</div>
