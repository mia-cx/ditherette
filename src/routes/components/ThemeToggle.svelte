<script lang="ts" module>
	const STORAGE_KEY = 'ditherette:theme';
	export type ThemeChoice = 'light' | 'dark';
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/ui/button';
	import SunIcon from 'phosphor-svelte/lib/Sun';
	import MoonIcon from 'phosphor-svelte/lib/Moon';

	let isDark = $state(false);

	function apply(dark: boolean) {
		isDark = dark;
		document.documentElement.classList.toggle('dark', dark);
		try {
			localStorage.setItem(STORAGE_KEY, dark ? 'dark' : 'light');
		} catch {
			// localStorage may be unavailable (private mode, quota); ignore.
		}
	}

	onMount(() => {
		// Sync the in-component flag with whatever the FOUC-prevention
		// script in app.html already applied to <html>.
		isDark = document.documentElement.classList.contains('dark');
	});
</script>

<Button
	variant="ghost"
	size="icon-sm"
	onclick={() => apply(!isDark)}
	aria-label={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
	aria-pressed={isDark}
	title={isDark ? 'Switch to light mode' : 'Switch to dark mode'}
>
	{#if isDark}
		<SunIcon weight="bold" />
	{:else}
		<MoonIcon weight="bold" />
	{/if}
</Button>
