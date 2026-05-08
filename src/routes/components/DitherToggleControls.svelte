<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Switch } from '$lib/components/ui/switch';

	type Props = {
		useColorSpace: boolean;
		serpentine: boolean;
		supportsColorSpaceDither: boolean;
		isErrorDiffusion: boolean;
	};

	let {
		useColorSpace = $bindable(),
		serpentine = $bindable(),
		supportsColorSpaceDither,
		isErrorDiffusion
	}: Props = $props();
</script>

<div class="grid gap-2">
	<div class="flex items-center justify-between gap-3">
		<Label for="dither-color-space" class="grid gap-0.5 text-left">
			<span>Dither in selected space</span>
			<span class="text-xs font-normal text-muted-foreground"
				>Perturb and diffuse error in the selected color space. Color matching always uses it.</span
			>
		</Label>
		<Switch
			id="dither-color-space"
			bind:checked={useColorSpace}
			disabled={!supportsColorSpaceDither}
		/>
	</div>

	<div class="flex items-center justify-between gap-3">
		<Label for="dither-serpentine" class="grid gap-0.5 text-left">
			<span>Serpentine scan</span>
			<span class="text-xs font-normal text-muted-foreground"
				>Reduces directional bias for error diffusion.</span
			>
		</Label>
		<Switch id="dither-serpentine" bind:checked={serpentine} disabled={!isErrorDiffusion} />
	</div>
</div>
