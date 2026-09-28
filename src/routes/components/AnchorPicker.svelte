<script lang="ts">
	import type { ResizeAnchor } from 'ditherette';

	type Props = {
		value: ResizeAnchor;
		disabled?: boolean;
		onchange: (anchor: ResizeAnchor) => void;
		labelledby: string;
	};
	let { value, disabled = false, onchange, labelledby }: Props = $props();

	/** Row-major, so arrow keys move by ±1 across and ±3 down. */
	const ANCHORS = [
		{ id: 'top-left', label: 'Top left' },
		{ id: 'top', label: 'Top' },
		{ id: 'top-right', label: 'Top right' },
		{ id: 'left', label: 'Left' },
		{ id: 'center', label: 'Center' },
		{ id: 'right', label: 'Right' },
		{ id: 'bottom-left', label: 'Bottom left' },
		{ id: 'bottom', label: 'Bottom' },
		{ id: 'bottom-right', label: 'Bottom right' }
	] as const satisfies readonly { id: ResizeAnchor; label: string }[];
	const STEPS: Record<string, [number, number]> = {
		ArrowLeft: [-1, 0],
		ArrowRight: [1, 0],
		ArrowUp: [0, -1],
		ArrowDown: [0, 1]
	};

	let group = $state<HTMLElement>();

	function move(event: KeyboardEvent, index: number) {
		const step = STEPS[event.key];
		if (!step) return;
		event.preventDefault();
		const column = Math.min(2, Math.max(0, (index % 3) + step[0]));
		const row = Math.min(2, Math.max(0, Math.floor(index / 3) + step[1]));
		const next = row * 3 + column;
		onchange(ANCHORS[next]!.id);
		group?.querySelectorAll<HTMLElement>('[role="radio"]')[next]?.focus();
	}
</script>

<div
	bind:this={group}
	class="grid w-fit grid-cols-3 gap-0.5 {disabled ? 'opacity-50' : ''}"
	role="radiogroup"
	aria-labelledby={labelledby}
	aria-disabled={disabled}
>
	{#each ANCHORS as anchor, index (anchor.id)}
		{@const checked = anchor.id === value}
		<button
			type="button"
			role="radio"
			aria-checked={checked}
			aria-label={anchor.label}
			title={anchor.label}
			tabindex={checked && !disabled ? 0 : -1}
			{disabled}
			class="grid size-6 place-items-center border border-input bg-background hover:bg-muted focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none disabled:pointer-events-none"
			onclick={() => onchange(anchor.id)}
			onkeydown={(event) => move(event, index)}
		>
			<span class="size-2 {checked ? 'bg-primary' : 'bg-muted-foreground/30'}"></span>
		</button>
	{/each}
</div>
