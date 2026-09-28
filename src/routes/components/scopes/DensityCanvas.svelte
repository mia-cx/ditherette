<script lang="ts">
	type Props = { image: ImageData };
	/** Draws a scope's painted density, smoothly scaled to fill its box. Nothing reads it back. */
	let { image }: Props = $props();

	let canvas = $state<HTMLCanvasElement>();
	let width = $state(0);
	let height = $state(0);

	const bitmap = $derived.by(() => {
		const scratch = document.createElement('canvas');
		scratch.width = image.width;
		scratch.height = image.height;
		scratch.getContext('2d')?.putImageData(image, 0, 0);
		return scratch;
	});

	$effect(() => {
		if (!canvas) return;
		const observer = new ResizeObserver(() => {
			const ratio = window.devicePixelRatio || 1;
			width = Math.round(canvas!.clientWidth * ratio);
			height = Math.round(canvas!.clientHeight * ratio);
		});
		observer.observe(canvas);
		return () => observer.disconnect();
	});

	$effect(() => {
		if (!canvas || !width || !height) return;
		canvas.width = width;
		canvas.height = height;
		const context = canvas.getContext('2d');
		if (!context) return;
		context.imageSmoothingEnabled = true;
		context.imageSmoothingQuality = 'high';
		context.drawImage(bitmap, 0, 0, width, height);
	});
</script>

<canvas bind:this={canvas} class="absolute inset-0 size-full" aria-hidden="true"></canvas>
