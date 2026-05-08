<script lang="ts">
	type Props = {
		fieldBackground: string;
		handleStyle: string;
		onPickPlane: (event: PointerEvent) => void;
	};

	let { fieldBackground, handleStyle, onPickPlane }: Props = $props();
</script>

<div class="grid">
	<button
		type="button"
		class="relative aspect-square min-h-52 touch-none overflow-hidden border border-border bg-transparent p-0"
		aria-label="Color field"
		onpointerdown={onPickPlane}
	>
		<span class="absolute -inset-px" style="background: {fieldBackground};"></span>
		<span
			class="absolute size-4 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={handleStyle}
		></span>
	</button>
</div>
