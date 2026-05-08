<script lang="ts">
	type Props = {
		fieldBackground: string;
		handleStyle: string;
		hueRailBackground: string;
		hue: number;
		onPickPlane: (event: PointerEvent) => void;
		onPickHueRail: (event: PointerEvent) => void;
	};

	let { fieldBackground, handleStyle, hueRailBackground, hue, onPickPlane, onPickHueRail }: Props =
		$props();
</script>

<div class="grid grid-cols-[minmax(0,1fr)_1.75rem] gap-3">
	<button
		type="button"
		class="relative aspect-square min-h-52 touch-none overflow-hidden border border-border bg-transparent p-0"
		aria-label="Color field"
		onpointerdown={onPickPlane}
	>
		<span class="absolute -inset-px" style="background: {fieldBackground};"></span>
		<span
			class="absolute size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={handleStyle}
		></span>
	</button>
	<button
		type="button"
		class="relative touch-none overflow-hidden border border-border bg-transparent p-0"
		aria-label="Hue rail"
		onpointerdown={onPickHueRail}
	>
		<span class="absolute -inset-px" style="background: {hueRailBackground};"></span>
		<span
			class="absolute left-1/2 h-1.5 w-8 -translate-x-1/2 -translate-y-1/2 border border-background bg-foreground shadow-sm"
			style="top: {(hue / 360) * 100}%;"
		></span>
	</button>
</div>
