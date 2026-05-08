<script lang="ts" module>
	let nextGradientId = 0;

	function allocateGradientId() {
		const id = nextGradientId;
		nextGradientId += 1;
		return id;
	}
</script>

<script lang="ts">
	import { onDestroy } from 'svelte';

	type Point = { x: number; y: number };
	type TriangleCommit = { point: Point; width: number };
	type Props = {
		hueWheelBackground: string;
		hue: number;
		saturation: number;
		value: number;
		hueHandleStyle: string;
		triangleHandleStyle: string;
		onPickHue: (hue: number) => void;
		onPickSaturationValue: (saturation: number, value: number) => void;
		onPickTriangle: (point: Point, width: number) => void;
	};

	let {
		hueWheelBackground,
		hue,
		saturation,
		value,
		hueHandleStyle,
		triangleHandleStyle,
		onPickHue,
		onPickSaturationValue,
		onPickTriangle
	}: Props = $props();
	let pendingHue: number | null = null;
	let pendingTriangle: TriangleCommit | null = null;
	let commitFrame: number | null = null;
	let activeDragCleanup: (() => void) | null = null;

	const triangleRadiusRatio = 0.25;
	const gradientId = `wheel-triangle-${allocateGradientId()}`;
	const whiteGradientId = `${gradientId}-white`;
	const blackGradientId = `${gradientId}-black`;

	onDestroy(() => {
		activeDragCleanup?.();
		if (commitFrame === null) return;
		cancelAnimationFrame(commitFrame);
		commitFrame = null;
		pendingHue = null;
		pendingTriangle = null;
	});

	function pickWheel(event: PointerEvent) {
		activeDragCleanup?.();
		const target = event.currentTarget as HTMLElement;
		const hueHandle = target.querySelector<HTMLElement>('[data-wheel-hue-handle]');
		const triangleHandle = target.querySelector<HTMLElement>('[data-wheel-triangle-handle]');
		const triangle = target.querySelector<HTMLElement>('[data-wheel-triangle]');
		const hueFill = target.querySelector<SVGPolygonElement>('[data-wheel-triangle-fill]');
		const dragMode = wheelDragMode(event, target);
		const update = (next: PointerEvent) => {
			const rect = target.getBoundingClientRect();
			const point = wheelPoint(next, rect);
			if (dragMode === 'hue') {
				const nextHue = hueFromWheelPoint(point.x, point.y);
				hueHandle?.setAttribute('style', wheelHandleStyle(nextHue));
				triangle?.style.setProperty('transform', `translate(-50%, -50%) rotate(${nextHue}deg)`);
				hueFill?.setAttribute('fill', `hsl(${nextHue} 100% 50%)`);
				scheduleHueCommit(nextHue);
				return;
			}
			const clamped = closestWheelTrianglePoint(point, rect.width, hue);
			const percent = {
				x: ((clamped.x + rect.width / 2) / rect.width) * 100,
				y: ((clamped.y + rect.height / 2) / rect.height) * 100
			};
			triangleHandle?.style.setProperty('left', `${percent.x}%`);
			triangleHandle?.style.setProperty('top', `${percent.y}%`);
			scheduleTriangleCommit({ point: clamped, width: rect.width });
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
			activeDragCleanup = null;
		};
		activeDragCleanup = () => cleanup();
		target.setPointerCapture(pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = cleanup;
		target.onpointercancel = cleanup;
		target.onlostpointercapture = cleanup;
		window.addEventListener('pointerup', cleanup);
		window.addEventListener('pointercancel', cleanup);
	}

	function handleWheelKeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
			event.preventDefault();
			const direction = event.key === 'ArrowRight' ? 1 : -1;
			onPickHue(wrapHue(hue + direction * (event.shiftKey ? 10 : 1)));
			return;
		}
		if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
			event.preventDefault();
			const direction = event.key === 'ArrowUp' ? 1 : -1;
			onPickSaturationValue(
				saturation,
				clamp(value + direction * (event.shiftKey ? 10 : 1), 0, 100)
			);
			return;
		}
		if (event.key === 'PageUp' || event.key === 'PageDown') {
			event.preventDefault();
			const direction = event.key === 'PageUp' ? 1 : -1;
			onPickSaturationValue(clamp(saturation + direction * 10, 0, 100), value);
			return;
		}
		if (event.key === 'Home') {
			event.preventDefault();
			onPickSaturationValue(0, value);
			return;
		}
		if (event.key === 'End') {
			event.preventDefault();
			onPickSaturationValue(100, value);
		}
	}

	function cleanupPointer(target: HTMLElement, pointerId: number) {
		if (target.hasPointerCapture(pointerId)) target.releasePointerCapture(pointerId);
		target.onpointermove = null;
		target.onpointerup = null;
		target.onpointercancel = null;
		target.onlostpointercapture = null;
	}

	function scheduleHueCommit(hue: number) {
		pendingHue = hue;
		scheduleCommitFrame();
	}

	function scheduleTriangleCommit(commit: TriangleCommit) {
		pendingTriangle = commit;
		scheduleCommitFrame();
	}

	function scheduleCommitFrame() {
		if (commitFrame !== null) return;
		commitFrame = requestAnimationFrame(() => {
			commitFrame = null;
			const hue = pendingHue;
			const triangle = pendingTriangle;
			pendingHue = null;
			pendingTriangle = null;
			if (hue !== null) onPickHue(hue);
			if (triangle) onPickTriangle(triangle.point, triangle.width);
		});
	}

	function wheelDragMode(event: PointerEvent, target: HTMLElement): 'hue' | 'triangle' {
		const rect = target.getBoundingClientRect();
		const point = wheelPoint(event, rect);
		return Math.hypot(point.x, point.y) >= (rect.width / 2) * 0.74 ? 'hue' : 'triangle';
	}

	function wheelPoint(event: PointerEvent, rect: DOMRect) {
		return {
			x: event.clientX - (rect.left + rect.width / 2),
			y: event.clientY - (rect.top + rect.height / 2)
		};
	}

	function hueFromWheelPoint(x: number, y: number) {
		return ((Math.atan2(y, x) * 180) / Math.PI + 360) % 360;
	}

	function wheelHandleStyle(nextHue: number) {
		const angle = (nextHue * Math.PI) / 180;
		return `left: ${50 + Math.cos(angle) * 43}%; top: ${50 + Math.sin(angle) * 43}%;`;
	}

	function closestWheelTrianglePoint(point: Point, width: number, nextHue: number) {
		const vertices = triangleVertices(width);
		const local = rotatePoint(point, -nextHue);
		const clamped = closestPointInTriangle(local, vertices.white, vertices.black, vertices.hue);
		return rotatePoint(clamped, nextHue);
	}

	function triangleVertices(width: number) {
		const radius = width * triangleRadiusRatio;
		return {
			white: { x: -radius / 2, y: -(Math.sqrt(3) / 2) * radius },
			black: { x: -radius / 2, y: (Math.sqrt(3) / 2) * radius },
			hue: { x: radius, y: 0 }
		};
	}

	function closestPointInTriangle(point: Point, white: Point, black: Point, hue: Point) {
		const weights = barycentric(point, white, black, hue);
		if (weights.white >= 0 && weights.black >= 0 && weights.hue >= 0) return point;
		return [
			closestPointOnSegment(point, white, black),
			closestPointOnSegment(point, black, hue),
			closestPointOnSegment(point, hue, white)
		].reduce((best, candidate) =>
			distanceSquared(point, candidate) < distanceSquared(point, best) ? candidate : best
		);
	}

	function barycentric(point: Point, white: Point, black: Point, hue: Point) {
		const denom = (black.y - hue.y) * (white.x - hue.x) + (hue.x - black.x) * (white.y - hue.y);
		const whiteWeight =
			((black.y - hue.y) * (point.x - hue.x) + (hue.x - black.x) * (point.y - hue.y)) / denom;
		const blackWeight =
			((hue.y - white.y) * (point.x - hue.x) + (white.x - hue.x) * (point.y - hue.y)) / denom;
		return { white: whiteWeight, black: blackWeight, hue: 1 - whiteWeight - blackWeight };
	}

	function closestPointOnSegment(point: Point, start: Point, end: Point) {
		const dx = end.x - start.x;
		const dy = end.y - start.y;
		const lengthSquared = dx * dx + dy * dy;
		const t =
			lengthSquared === 0
				? 0
				: clamp(((point.x - start.x) * dx + (point.y - start.y) * dy) / lengthSquared, 0, 1);
		return { x: start.x + t * dx, y: start.y + t * dy };
	}

	function rotatePoint(point: Point, degrees: number) {
		const radians = (degrees * Math.PI) / 180;
		const cos = Math.cos(radians);
		const sin = Math.sin(radians);
		return { x: point.x * cos - point.y * sin, y: point.x * sin + point.y * cos };
	}

	function distanceSquared(left: Point, right: Point) {
		return (left.x - right.x) ** 2 + (left.y - right.y) ** 2;
	}

	function clamp(value: number, min: number, max: number) {
		return Math.min(max, Math.max(min, value));
	}

	function wrapHue(value: number) {
		return (value + 360) % 360;
	}
</script>

<div class="grid place-items-center p-2">
	<button
		type="button"
		class="relative size-72 touch-none overflow-hidden rounded-full border-0 bg-transparent p-0"
		aria-label="Hue wheel. Arrow left and right adjust hue, arrow up and down adjust value, PageUp and PageDown adjust saturation."
		onpointerdown={pickWheel}
		onkeydown={handleWheelKeydown}
	>
		<span class="absolute -inset-px rounded-full" style="background: {hueWheelBackground};"></span>
		<span class="absolute inset-[13%] rounded-full bg-background"></span>
		<span
			data-wheel-triangle
			class="absolute top-1/2 left-1/2 size-36 overflow-hidden"
			style="transform: translate(-50%, -50%) rotate({hue}deg);"
		>
			<svg class="absolute inset-0 size-full" viewBox="0 0 100 100" aria-hidden="true">
				<defs>
					<linearGradient
						id={whiteGradientId}
						gradientUnits="userSpaceOnUse"
						x1="25"
						y1="6.699"
						x2="62.5"
						y2="71.651"
					>
						<stop offset="0" stop-color="#fff" />
						<stop offset="1" stop-color="#fff" stop-opacity="0" />
					</linearGradient>
					<linearGradient
						id={blackGradientId}
						gradientUnits="userSpaceOnUse"
						x1="25"
						y1="93.301"
						x2="62.5"
						y2="28.349"
					>
						<stop offset="0" stop-color="#000" />
						<stop offset="1" stop-color="#000" stop-opacity="0" />
					</linearGradient>
				</defs>
				<polygon
					data-wheel-triangle-fill
					points="100,50 25,6.699 25,93.301"
					fill="hsl({hue} 100% 50%)"
				/>
				<polygon points="100,50 25,6.699 25,93.301" fill="url(#{whiteGradientId})" />
				<polygon points="100,50 25,6.699 25,93.301" fill="url(#{blackGradientId})" />
			</svg>
		</span>
		<span
			data-wheel-hue-handle
			class="absolute size-4 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={hueHandleStyle}
		></span>
		<span
			data-wheel-triangle-handle
			class="absolute size-3 -translate-x-1/2 -translate-y-1/2 border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
			style={triangleHandleStyle}
		></span>
	</button>
</div>
