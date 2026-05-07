<script lang="ts">
	import katex from 'katex';
	import 'katex/dist/katex.min.css';

	type Props = { expression: string; displayMode?: boolean };
	let { expression, displayMode = false }: Props = $props();

	const html = $derived(
		katex.renderToString(expression, {
			displayMode,
			throwOnError: false,
			strict: false,
			trust: false
		})
	);
</script>

<span
	class="block max-w-full overflow-hidden text-[0.72rem] leading-relaxed break-words text-muted-foreground [&_.katex]:max-w-full [&_.katex]:text-[1em] [&_.katex]:whitespace-normal"
>
	<!-- eslint-disable-next-line svelte/no-at-html-tags -- safe: KaTeX renders escaped math with trust disabled. -->
	{@html html}
</span>
