<script lang="ts">
	type Value = { label: string; value: string };
	type Props = {
		description: string;
		values: Value[];
	};

	let { description, values }: Props = $props();
</script>

<div class="grid gap-3 py-3 text-sm">
	<p class="text-muted-foreground">{description}</p>
	<div class="grid grid-cols-3 gap-2 font-mono text-xs tabular-nums">
		{#each values as item (item.label)}
			<span>{item.label} {item.value}</span>
		{/each}
	</div>
</div>
