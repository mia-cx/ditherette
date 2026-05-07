<script lang="ts">
	import type { SVGAttributes } from 'svelte/elements';

	let { class: className, ...restProps }: SVGAttributes<SVGSVGElement> = $props();
</script>

<svg viewBox="0 0 256 256" fill="none" class={className} aria-hidden="true" {...restProps}>
	<path
		d="M100 68c2.122 0 4.157.843 5.657 2.343S108 73.878 108 76v104c0 2.122-.843 4.157-2.343 5.657S102.122 188 100 188H56c-2.122 0-4.157-.843-5.657-2.343S48 182.122 48 180V76c0-2.122.843-4.157 2.343-5.657S53.878 68 56 68h44Z"
		stroke="currentColor"
		stroke-width="24"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
	<path
		d="M200 68c2.122 0 4.157.843 5.657 2.343S208 73.878 208 76v104c0 2.122-.843 4.157-2.343 5.657S202.122 188 200 188h-44c-2.122 0-4.157-.843-5.657-2.343S148 182.122 148 180V76c0-2.122.843-4.157 2.343-5.657S153.878 68 156 68h44Z"
		stroke="currentColor"
		stroke-width="24"
		stroke-linecap="round"
		stroke-linejoin="round"
	/>
</svg>
