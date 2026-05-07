<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import { COLOR_SPACES } from './sample-data';
	import { colorSpace } from '$lib/stores/app';

	type Props = { compact?: boolean; hideHeading?: boolean };
	let { compact = false, hideHeading = false }: Props = $props();

	let canvas = $state<HTMLCanvasElement>();
	let space = $state(colorSpace.get());
	const current = $derived(COLOR_SPACES.find((s) => s.id === space) ?? COLOR_SPACES[0]);
	const triggerLabel = $derived(current.label);

	$effect(() => {
		colorSpace.set(space);
	});

	$effect(() => {
		if (!canvas) return;
		drawVisualizer(canvas, space);
	});

	function drawVisualizer(target: HTMLCanvasElement, mode: string) {
		const scale = window.devicePixelRatio || 1;
		const width = Math.max(1, Math.round(target.clientWidth * scale));
		const height = Math.max(1, Math.round(target.clientHeight * scale));
		if (target.width !== width || target.height !== height) {
			target.width = width;
			target.height = height;
		}
		const context = target.getContext('2d');
		if (!context) return;
		context.imageSmoothingEnabled = true;
		context.imageSmoothingQuality = 'high';
		context.clearRect(0, 0, width, height);
		context.fillStyle = 'hsl(220 15% 9%)';
		context.fillRect(0, 0, width, height);

		const points = 17;
		for (let l = 0; l < points; l++) {
			for (let c = 0; c < points; c++) {
				const hue = ((c / (points - 1)) * 320 + l * 9) % 360;
				const radius = (c / (points - 1)) * Math.min(width, height) * 0.34;
				const angle = (hue / 180) * Math.PI;
				const depth = l / (points - 1);
				const x = width * 0.5 + Math.cos(angle) * radius + (depth - 0.5) * width * 0.22;
				const y = height * 0.62 + Math.sin(angle) * radius * 0.55 - depth * height * 0.42;
				const size = Math.max(2, 4 * scale + depth * 2 * scale);
				context.fillStyle = colorForPoint(mode, hue, depth, c / (points - 1));
				context.globalAlpha = 0.55 + depth * 0.4;
				context.beginPath();
				context.arc(x, y, size, 0, Math.PI * 2);
				context.fill();
			}
		}
		context.globalAlpha = 1;
		context.strokeStyle = 'rgba(255,255,255,0.5)';
		context.lineWidth = scale;
		context.beginPath();
		context.moveTo(width * 0.14, height * 0.82);
		context.lineTo(width * 0.82, height * 0.82);
		context.moveTo(width * 0.14, height * 0.82);
		context.lineTo(width * 0.26, height * 0.2);
		context.moveTo(width * 0.14, height * 0.82);
		context.lineTo(width * 0.04, height * 0.58);
		context.stroke();
		context.fillStyle = 'rgba(255,255,255,0.8)';
		context.font = `${11 * scale}px sans-serif`;
		context.fillText(axisLabels(mode)[0], width * 0.84, height * 0.84);
		context.fillText(axisLabels(mode)[1], width * 0.27, height * 0.18);
		context.fillText(axisLabels(mode)[2], width * 0.03, height * 0.55);
	}

	function colorForPoint(mode: string, hue: number, lightness: number, chroma: number) {
		if (mode === 'srgb' || mode.startsWith('weighted')) {
			return `rgb(${Math.round(lightness * 255)} ${Math.round(chroma * 255)} ${Math.round((1 - lightness) * 255)})`;
		}
		return `oklch(${0.35 + lightness * 0.5} ${0.05 + chroma * 0.22} ${hue})`;
	}

	function axisLabels(mode: string) {
		if (mode === 'oklch') return ['hue', 'L', 'C'];
		if (mode === 'cielab') return ['a*', 'L*', 'b*'];
		if (mode.includes('rgb')) return ['R', 'G', 'B'];
		return ['a', 'L', 'b'];
	}
</script>

<section class="flex flex-col gap-{compact ? '3' : '4'}" aria-label="Color space controls">
	{#if !hideHeading}
		<div class="flex items-baseline justify-between gap-2">
			<h2 class="text-sm font-semibold tracking-tight">Color space</h2>
			<p class="text-xs text-muted-foreground">How nearest-color is computed.</p>
		</div>
	{/if}

	<div class="grid gap-1.5">
		<Label for="color-space">Distance mode</Label>
		<Select bind:value={space} type="single">
			<SelectTrigger id="color-space">{triggerLabel}</SelectTrigger>
			<SelectContent>
				{#each COLOR_SPACES as s (s.id)}
					<SelectItem value={s.id}>{s.label}</SelectItem>
				{/each}
			</SelectContent>
		</Select>
	</div>

	<figure class="relative aspect-[4/3] w-full overflow-hidden border border-border bg-background">
		<canvas
			bind:this={canvas}
			class="absolute inset-0 h-full w-full [image-rendering:auto]"
			aria-label="{current.label} 3D canvas visualizer"
		></canvas>
		<figcaption class="absolute right-2 bottom-2 bg-background/85 px-2 py-0.5 text-xs">
			{current.label}
		</figcaption>
	</figure>

	<div class="grid gap-1.5">
		<p class="text-sm leading-relaxed">{current.short}</p>
		<p
			class="border-l-2 border-border bg-muted/50 px-3 py-1.5 font-mono text-xs text-muted-foreground"
		>
			{current.math}
		</p>
	</div>
</section>
