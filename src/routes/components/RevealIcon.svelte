<script lang="ts">
	import type { SVGAttributes } from 'svelte/elements';

	let { class: className, ...restProps }: SVGAttributes<SVGSVGElement> = $props();
</script>

<svg viewBox="0 0 256 256" fill="none" class={className} aria-hidden="true" {...restProps}>
	<path
		d="M118 188H48V68h70"
		stroke="currentColor"
		stroke-width="24"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M118 48v160"
		stroke="currentColor"
		stroke-width="24"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M148 68h60v120h-60"
		stroke="currentColor"
		stroke-width="24"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
</svg>
