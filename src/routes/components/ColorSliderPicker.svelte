<script lang="ts">
	type Channel = {
		label: string;
		value: number;
		min: number;
		max: number;
		step?: number;
		background: string;
		onChange: (value: number) => void;
	};

	type Props = {
		channels: Channel[];
	};

	let { channels }: Props = $props();

	function pickChannel(event: PointerEvent, channel: Channel) {
		const target = event.currentTarget as HTMLElement;
		const step = channel.step ?? 1;
		const update = (next: PointerEvent) => {
			const rect = target.getBoundingClientRect();
			const ratio = Math.min(1, Math.max(0, (next.clientX - rect.left) / rect.width));
			const raw = channel.min + ratio * (channel.max - channel.min);
			channel.onChange(Math.round(raw / step) * step);
		};
		target.setPointerCapture(event.pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = () => (target.onpointermove = null);
	}

	function percent(channel: Channel) {
		return ((channel.value - channel.min) / (channel.max - channel.min)) * 100;
	}
</script>

<div class="grid gap-2">
	{#each channels as channel (channel.label)}
		<label class="grid grid-cols-[1.5rem_minmax(0,1fr)] items-center gap-2 text-xs">
			<span class="text-muted-foreground">{channel.label}</span>
			<button
				type="button"
				class="relative h-3 touch-none overflow-visible border border-border bg-transparent p-0"
				aria-label="{channel.label} channel"
				onpointerdown={(event) => pickChannel(event, channel)}
			>
				<span class="absolute -inset-px" style="background: {channel.background};"></span>
				<span
					class="absolute top-1/2 size-3.5 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.75)]"
					style="left: {percent(channel)}%;"
				></span>
			</button>
		</label>
	{/each}
</div>
