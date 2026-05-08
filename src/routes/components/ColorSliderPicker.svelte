<script lang="ts">
	import { onDestroy } from 'svelte';

	type Channel = {
		label: string;
		value: number;
		min: number;
		max: number;
		step?: number;
		background: string;
		onChange: (value: number) => void;
	};

	type PendingCommit = { channel: Channel; value: number };
	type Props = {
		channels: Channel[];
	};

	let { channels }: Props = $props();
	let pendingCommit: PendingCommit | null = null;
	let commitFrame: number | null = null;

	onDestroy(() => {
		if (commitFrame === null) return;
		cancelAnimationFrame(commitFrame);
		commitFrame = null;
		pendingCommit = null;
	});

	function pickChannel(event: PointerEvent, channel: Channel) {
		const target = event.currentTarget as HTMLElement;
		const handle = target.querySelector<HTMLElement>('[data-slider-handle]');
		const update = (next: PointerEvent) => {
			const ratio = ratioFromPointer(next, target);
			setHandlePercent(handle, ratio * 100);
			scheduleCommit({ channel, value: valueFromRatio(channel, ratio) });
		};
		const pointerId = event.pointerId;
		let cleanedUp = false;
		const cleanup = (next?: PointerEvent) => {
			if (next && next.pointerId !== pointerId) return;
			if (cleanedUp) return;
			cleanedUp = true;
			window.removeEventListener('pointerup', cleanup);
			window.removeEventListener('pointercancel', cleanup);
			cleanupPointer(target, pointerId);
		};
		target.setPointerCapture(pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = cleanup;
		target.onpointercancel = cleanup;
		target.onlostpointercapture = cleanup;
		window.addEventListener('pointerup', cleanup);
		window.addEventListener('pointercancel', cleanup);
	}

	function handleSliderKeydown(event: KeyboardEvent, channel: Channel) {
		const keySteps: Record<string, number> = {
			ArrowLeft: -1,
			ArrowDown: -1,
			ArrowRight: 1,
			ArrowUp: 1,
			PageDown: -10,
			PageUp: 10
		};
		if (event.key === 'Home') {
			event.preventDefault();
			scheduleCommit({ channel, value: channel.min });
			return;
		}
		if (event.key === 'End') {
			event.preventDefault();
			scheduleCommit({ channel, value: channel.max });
			return;
		}
		const direction = keySteps[event.key];
		if (!direction) return;
		event.preventDefault();
		scheduleCommit({
			channel,
			value: snapValue(channel, channel.value + direction * (channel.step ?? 1))
		});
	}

	function scheduleCommit(next: PendingCommit) {
		pendingCommit = next;
		if (commitFrame !== null) return;
		commitFrame = requestAnimationFrame(() => {
			commitFrame = null;
			const commit = pendingCommit;
			pendingCommit = null;
			commit?.channel.onChange(commit.value);
		});
	}

	function cleanupPointer(target: HTMLElement, pointerId: number) {
		if (target.hasPointerCapture(pointerId)) target.releasePointerCapture(pointerId);
		target.onpointermove = null;
		target.onpointerup = null;
		target.onpointercancel = null;
		target.onlostpointercapture = null;
	}

	function ratioFromPointer(event: PointerEvent, target: HTMLElement) {
		const rect = target.getBoundingClientRect();
		if (rect.width === 0) return 0;
		return clamp((event.clientX - rect.left) / rect.width, 0, 1);
	}

	function valueFromRatio(channel: Channel, ratio: number) {
		return snapValue(channel, channel.min + ratio * (channel.max - channel.min));
	}

	function snapValue(channel: Channel, value: number) {
		const step = channel.step ?? 1;
		return clamp(
			Math.round((value - channel.min) / step) * step + channel.min,
			channel.min,
			channel.max
		);
	}

	function setHandlePercent(handle: HTMLElement | null, nextPercent: number) {
		handle?.style.setProperty('left', `${clamp(nextPercent, 0, 100)}%`);
	}

	function percent(channel: Channel) {
		const range = channel.max - channel.min;
		if (range === 0) return 0;
		return clamp(((channel.value - channel.min) / range) * 100, 0, 100);
	}

	function clamp(value: number, min: number, max: number) {
		return Math.min(max, Math.max(min, value));
	}
</script>

<div class="grid gap-2">
	{#each channels as channel, index (index)}
		<label class="grid grid-cols-[1.5rem_minmax(0,1fr)] items-center gap-2 text-xs">
			<span class="text-muted-foreground">{channel.label}</span>
			<button
				type="button"
				class="relative h-3 touch-none overflow-visible border border-border bg-transparent p-0"
				role="slider"
				aria-label="{channel.label} channel"
				aria-valuemin={channel.min}
				aria-valuemax={channel.max}
				aria-valuenow={channel.value}
				aria-orientation="horizontal"
				tabindex="0"
				onpointerdown={(event) => pickChannel(event, channel)}
				onkeydown={(event) => handleSliderKeydown(event, channel)}
			>
				<span class="absolute -inset-px" style="background: {channel.background};"></span>
				<span
					data-slider-handle
					class="absolute top-1/2 size-3.5 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.75)]"
					style="left: {percent(channel)}%;"
				></span>
			</button>
		</label>
	{/each}
</div>
