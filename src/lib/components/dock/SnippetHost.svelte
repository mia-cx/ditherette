<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { DockParams } from './dock';

	/** Renders a caller's snippet inside a window Dockview created outside the component tree. */
	let { snippet, params }: { snippet: Snippet<[DockParams]>; params: DockParams } = $props();
</script>

{@render snippet(params)}
